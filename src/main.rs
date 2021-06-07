mod async_data;
mod endpoint;
mod error;
mod schema;
mod types;

pub use error::{Error, Result};

use std::fmt::Display;
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
use druid::widget::{Controller, Flex, Label, List};
use druid::{
    AppLauncher, Data, Env, Event, EventCtx, FontDescriptor, FontFamily, Lens, LifeCycle,
    LifeCycleCtx, Widget, WidgetExt, WindowDesc,
};

fn main() -> Result<()> {
    let (tx, rx) = mpsc::unbounded_channel();

    let rt = Runtime::new().expect("Failed to create tokio runtime");
    let bg = rt.spawn(async_main(rx));

    let main_window = WindowDesc::new(move || manga_list(tx))
        .window_size((200., 200.))
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
}

fn arc_to_string<D: Display>(data: &Arc<D>, _env: &Env) -> String {
    data.to_string()
}
fn arc_to_owned<D: AsRef<str>>(data: &Arc<D>, _env: &Env) -> String {
    (**data).as_ref().to_owned()
}

fn manga_view() -> impl Widget<MangaViewData> {
    const FONT: FontDescriptor = FontDescriptor::new(FontFamily::MONOSPACE);
    let id_label = Label::dynamic(arc_to_string)
        .with_font(FONT)
        .lens(MangaViewData::id);
    let title_label = Label::dynamic(arc_to_owned)
        .with_font(FONT)
        .lens(MangaViewData::title);
    Flex::row().with_child(id_label).with_child(title_label)
}

#[derive(Default, Clone, druid::Data, druid::Lens)]
struct MangaListData {
    titles: im::Vector<MangaViewData>,
}

fn manga_list(tx: mpsc::UnboundedSender<Message>) -> impl Widget<MangaListData> {
    List::new(manga_view)
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
