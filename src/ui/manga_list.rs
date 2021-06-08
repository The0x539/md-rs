use crate::{async_data::AsyncData, endpoint, schema, types, Message, Result};

use std::sync::Arc;

use tokio::sync::mpsc;

use druid::im;
use druid::widget::{Controller, List};
use druid::{Env, Event, EventCtx, LifeCycle, LifeCycleCtx, Widget, WidgetExt};

use super::manga_view::{manga_view, MangaViewData};
use super::REFRESH;

#[derive(Default, Clone, druid::Data, druid::Lens)]
pub struct MangaListData {
    titles: im::Vector<MangaViewData>,
}

pub fn manga_list(tx: mpsc::UnboundedSender<Message>) -> impl Widget<MangaListData> {
    let tx_clone = tx.clone();
    List::new(move || manga_view(tx_clone.clone()))
        .horizontal()
        .with_spacing(4.0)
        .lens(MangaListData::titles)
        .controller(MangaListController::new(tx))
}

struct MangaListController {
    listing_info: AsyncData<Result<types::MangaList>>,
    tx: mpsc::UnboundedSender<Message>,
}

impl MangaListController {
    pub fn new(tx: mpsc::UnboundedSender<Message>) -> Self {
        Self {
            listing_info: Default::default(),
            tx,
        }
    }
}

impl<W: Widget<MangaListData>> Controller<MangaListData, W> for MangaListController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx<'_, '_>,
        event: &Event,
        data: &mut MangaListData,
        env: &Env,
    ) {
        if matches!(event, Event::Timer(_)) {
            if let Some(response) = self.listing_info.poll() {
                for item in response.unwrap().series.values() {
                    let series = MangaViewData {
                        id: item.id.into(),
                        title: item.attributes.title["en"].clone().into(),
                        cover_id: item
                            .relationships
                            .get(&types::RelationshipType::CoverArt)
                            .map(|id| Arc::new(schema::CoverId(*id))),
                        cover_buf: Arc::new(None),
                    };
                    data.titles.push_back(series);
                }
            } else {
                ctx.request_timer(REFRESH);
            }
        }
        child.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx<'_, '_>,
        event: &LifeCycle,
        data: &MangaListData,
        env: &Env,
    ) {
        if matches!(event, LifeCycle::WidgetAdded) {
            let fut = endpoint::get_json("https://api.mangadex.org/manga");
            self.listing_info.start(&self.tx, fut);
            ctx.request_timer(REFRESH);
        }
        child.lifecycle(ctx, event, data, env);
    }
}
