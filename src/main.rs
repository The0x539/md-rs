mod endpoint;
mod error;
mod schema;
pub use error::{Error, Result};

#[tokio::main(worker_threads = 5)]
async fn real_main() -> Result<()> {
    let id = {
        let resp: schema::MangaListResponse =
            endpoint::get_json("https://api.mangadex.org/manga").await?;
        resp.results[0].data.id
    };

    let chapter = {
        let url = format!("https://api.mangadex.org/manga/{}/feed", id);
        let resp: schema::MangaFeedResponse = endpoint::get_json(url).await?;
        resp.results[0].data.clone()
    };

    let endpoint = {
        let url = format!("https://api.mangadex.org/at-home/server/{}", &chapter.id);
        let resp: schema::BaseUrl = endpoint::get_json(url).await?;
        resp.base_url
    };

    let img_data = endpoint::get_image(
        &endpoint,
        "data",
        &chapter.attributes.hash,
        &chapter.attributes.data[1],
    )
    .await?;

    std::fs::write("foo.png", img_data).unwrap();

    Ok(())
}

fn main() {
    if let Err(e) = real_main() {
        println!("Error: {}", e);
    }
}
