use crate::{async_data::AsyncData, endpoint, schema, Message, Result};

use std::sync::Arc;

use tokio::sync::mpsc;

use druid::im;
use druid::piet::ImageFormat;
use druid::widget::{Controller, Flex, Image, Label};
use druid::{
    Data, Env, Event, EventCtx, ImageBuf, Lens, LifeCycle, LifeCycleCtx, UpdateCtx, Widget,
    WidgetExt,
};

use super::REFRESH;

#[derive(Clone, Data, Lens)]
pub struct MangaViewData {
    pub(super) id: Arc<schema::MangaId>,
    pub(super) title: Arc<String>,
    pub(super) cover_id: Option<Arc<schema::CoverId>>,
    pub(super) cover_buf: Arc<Option<ImageBuf>>,
}

fn arc_to_owned<D: AsRef<str>>(data: &Arc<D>, _env: &Env) -> String {
    (**data).as_ref().to_owned()
}

pub fn manga_view(tx: mpsc::UnboundedSender<Message>) -> impl Widget<MangaViewData> {
    let title_label = Label::dynamic(arc_to_owned).lens(MangaViewData::title);
    Flex::column()
        .with_child(title_label)
        .controller(MangaViewController::new(tx))
}

struct MangaViewController {
    cover_info: AsyncData<Result<image::RgbImage>>,
    tx: mpsc::UnboundedSender<Message>,
}

impl MangaViewController {
    pub fn new(tx: mpsc::UnboundedSender<Message>) -> Self {
        Self {
            cover_info: Default::default(),
            tx,
        }
    }
}

impl Controller<MangaViewData, Flex<MangaViewData>> for MangaViewController {
    fn event(
        &mut self,
        child: &mut Flex<MangaViewData>,
        ctx: &mut EventCtx<'_, '_>,
        event: &Event,
        data: &mut MangaViewData,
        env: &Env,
    ) {
        if matches!(event, Event::Timer(_)) {
            if let Some(response) = self.cover_info.poll() {
                let img = response.unwrap();
                let (w, h) = (img.width(), img.height());
                let pixels: Arc<[u8]> = img.into_raw().into();
                let buf = ImageBuf::from_raw(pixels, ImageFormat::Rgb, w as usize, h as usize);
                data.cover_buf = Arc::new(Some(buf));
            } else {
                ctx.request_timer(REFRESH);
            }
        }
        child.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        child: &mut Flex<MangaViewData>,
        ctx: &mut LifeCycleCtx<'_, '_>,
        event: &LifeCycle,
        data: &MangaViewData,
        env: &Env,
    ) {
        if matches!(event, LifeCycle::WidgetAdded) {
            if let Some(cover_id) = &data.cover_id {
                let url = format!("https://api.mangadex.org/cover/{}", cover_id);
                let manga_id = *data.id;
                let fut = async move {
                    let resp = endpoint::get_json::<_, schema::CoverResponse>(url).await?;

                    assert_eq!(resp.result, schema::Success::Ok);
                    assert_eq!(resp.data.item_type, schema::ItemType::CoverArt);

                    let filename = resp.data.attributes.file_name;

                    let img = endpoint::get_cover(&manga_id, &filename, ".256.jpg").await?;
                    Ok(img)
                };
                self.cover_info.start(&self.tx, fut);
                ctx.request_timer(REFRESH);
            }
        }
        child.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        child: &mut Flex<MangaViewData>,
        ctx: &mut UpdateCtx<'_, '_>,
        old_data: &MangaViewData,
        data: &MangaViewData,
        env: &Env,
    ) {
        child.update(ctx, old_data, data, env);
        // I do not understand why this goes *after* the other thing.
        if let (Some(buf), None) = (&*data.cover_buf, &*old_data.cover_buf) {
            let view = Image::new(buf.clone());
            child.add_child(view);
            ctx.children_changed();
        }
    }
}

#[derive(Default, Clone, druid::Data, druid::Lens)]
pub struct MangaListData {
    titles: im::Vector<MangaViewData>,
}
