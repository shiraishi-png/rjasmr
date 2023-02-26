use async_std::io::WriteExt;
use scraper::Html;
use scraper::Selector;
use std::{env, io::Result};

#[async_std::main]
async fn main() -> Result<()> {
    // If no arguments are passed, print the help message
    if env::args().len() == 1 {
        println!("Usage: rj <link1> <link2> ...");
        std::process::exit(1);
    }

    for link in env::args().skip(1) {
        let audio_links = get_audiolinks_from_rj(&link).await?;
        for audio_link in audio_links {
            let filename = audio_link.split("/").last().unwrap();
            // Check if the file already exists
            if async_std::fs::metadata(filename).await.is_ok() {
                println!("{} already exists", filename);
                continue;
            }

            let mut response = reqwest::Client::new()
                .get(&audio_link)
                .send()
                .await
                .unwrap();
            let mut file = async_std::fs::File::create(filename).await?;
            while let Some(chunk) = response.chunk().await.unwrap() {
                file.write_all(&chunk).await.unwrap();
            }
            println!("Downloaded {}", filename);
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
