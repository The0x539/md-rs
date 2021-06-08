#![deny(rust_2018_idioms)]

mod async_data;
mod endpoint;
mod error;
mod schema;
mod types;
mod ui;

pub use error::{Error, Result};

use futures::{
    future::BoxFuture,
    stream::{FuturesUnordered, StreamExt},
};
use snafu::ResultExt;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use std::path::PathBuf;

use druid::{AppLauncher, WindowDesc};

fn data_dir() -> PathBuf {
    let p_dirs = directories::ProjectDirs::from("", "The0x539", "md-rs")
        .expect("Could not determine project directory");

    let dir = p_dirs.data_dir();
    std::fs::create_dir_all(dir).expect("Failed to ensure existence of project directory");
    dir.to_owned()
}

fn main() -> Result<()> {
    let (tx, rx) = mpsc::unbounded_channel();

    let rt = Runtime::new().expect("Failed to create tokio runtime");
    let bg = rt.spawn(async_main(rx));

    let main_window = WindowDesc::new(move || ui::manga_list::manga_list(tx))
        .window_size((800., 300.))
        .set_position((100., 100.));
    let data = ui::manga_list::MangaListData::default();

    let launcher = AppLauncher::with_window(main_window);
    #[cfg(debug_assertions)]
    let launcher = launcher.use_simple_logger();

    launcher
        .launch(data)
        .context(error::DruidErr { action: "launch" })?;

    rt.block_on(bg).expect("Async half died");
    Ok(())
}

pub enum Message {
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
