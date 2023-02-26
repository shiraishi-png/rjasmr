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

            let reqwest_result = reqwest::Client::new()
                .get(&audio_link)
                .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/110.0")
                .header("Accept", "audio/webm,audio/ogg,audio/wav,audio/*;q=0.9,application/ogg;q=0.7,video/*;q=0.6,*/*;q=0.5")
                .header("Accept-Language", "en-US,en;q=0.5")
                .header("Range", "bytes=0-")
                .header("Alt-Used", "v.weeab0o.xyz")
                .header("Connection", "keep-alive")
                .header("Referer", "https://japaneseasmr.com/")
                .header("Sec-Fetch-Dest", "audio")
                .header("Sec-Fetch-Mode", "no-cors")
                .header("Sec-Fetch-Site", "cross-site")
                .header("DNT", "1")
                .header("Sec-GPC", "1")
                .header("Accept-Encoding", "identity")
                .header("TE", "trailers")
                .send()
                .await;
            let mut response = match reqwest_result {
                Ok(response) => response,
                Err(_) => {
                    println!("Error downloading {}", filename);
                    continue;
                }
            };

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
