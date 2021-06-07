#![deny(rust_2018_idioms)]

mod async_data;
mod endpoint;
mod error;
mod schema;
mod types;

pub use error::{Error, Result};

use std::sync::Arc;
use std::time::Duration;

use futures::{
    future::BoxFuture,
    stream::{FuturesUnordered, StreamExt},
};
use snafu::ResultExt;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use druid::im;
use druid::piet::ImageFormat;
use druid::widget::{Controller, Flex, Image, Label, List};
use druid::{
    AppLauncher, Data, Env, Event, EventCtx, ImageBuf, Lens, LifeCycle, LifeCycleCtx, UpdateCtx,
    Widget, WidgetExt, WindowDesc,
};

fn main() -> Result<()> {
    let (tx, rx) = mpsc::unbounded_channel();

    let rt = Runtime::new().expect("Failed to create tokio runtime");
    let bg = rt.spawn(async_main(rx));

    let main_window = WindowDesc::new(move || manga_list(tx))
        .window_size((800., 300.))
        .set_position((100., 100.));
    let data = MangaListData::default();

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .context(error::DruidErr { action: "launch" })?;

    rt.block_on(bg).expect("Async half died");
    Ok(())
}

#[derive(Clone, Data, Lens)]
struct MangaViewData {
    id: Arc<schema::MangaId>,
    title: Arc<String>,
    cover_id: Option<Arc<schema::CoverId>>,
    cover_buf: Arc<Option<ImageBuf>>,
}

fn arc_to_owned<D: AsRef<str>>(data: &Arc<D>, _env: &Env) -> String {
    (**data).as_ref().to_owned()
}

fn manga_view(tx: mpsc::UnboundedSender<Message>) -> impl Widget<MangaViewData> {
    let title_label = Label::dynamic(arc_to_owned).lens(MangaViewData::title);
    Flex::column()
        .with_child(title_label)
        .controller(MangaViewController::new(tx))
}

struct MangaViewController {
    cover_info: async_data::AsyncData<Result<image::RgbImage>>,
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
struct MangaListData {
    titles: im::Vector<MangaViewData>,
}

fn manga_list(tx: mpsc::UnboundedSender<Message>) -> impl Widget<MangaListData> {
    let tx_clone = tx.clone();
    List::new(move || manga_view(tx_clone.clone()))
        .horizontal()
        .with_spacing(4.0)
        .lens(MangaListData::titles)
        .controller(MangaListController::new(tx))
}

struct MangaListController {
    listing_info: async_data::AsyncData<Result<types::MangaList>>,
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

const REFRESH: Duration = Duration::from_millis(250);

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

pub enum Message {
    #[allow(unused)]
    Execute(BoxFuture<'static, ()>),
    //Error(Error),
}

async fn async_main(mut rx: mpsc::UnboundedReceiver<Message>) {
    let futs = FuturesUnordered::new();
    while let Some(msg) = rx.recv().await {
        match msg {
            Message::Execute(fut) => {
                let handle = tokio::spawn(fut);
                futs.push(handle);
            } //Message::Error(e) => println!("Background error: {}", e),
        }
    }
    futs.for_each_concurrent(None, |_| async {}).await;
}
