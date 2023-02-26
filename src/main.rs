use async_std::io::WriteExt;
use scraper::Html;
use scraper::Selector;
use std::{env, io::Result};

#[async_std::main]
async fn main() -> Result<()> {
    for link in env::args().skip(1) {
        let audio_links = get_audiolinks_from_rj(&link).await?;
        for audio_link in audio_links {
            let filename = audio_link.split("/").last().unwrap();
            let mut response = reqwest::Client::new()
                .get(&audio_link)
                .send()
                .await
                .unwrap();
            let mut file = async_std::fs::File::create(filename).await?;
            while let Some(chunk) = response.chunk().await.unwrap() {
                file.write_all(&chunk).await.unwrap();
            }
        }
    }
    Ok(())
}

async fn get_audiolinks_from_rj(url: &str) -> Result<Vec<String>> {
    let response = reqwest::get(url).await.unwrap();
    let body = response.text().await.unwrap();
    let document = Html::parse_document(&body);

    let selector = Selector::parse("audio>source").unwrap();
    let audio_urls = document
        .select(&selector)
        .map(|e| e.value().attr("src").unwrap().to_string())
        .collect::<Vec<String>>();

    Ok(audio_urls)
}
