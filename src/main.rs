use async_std::io::WriteExt;
use scraper::Html;
use scraper::Selector;
use std::{env, error::Error, io::Result};
use rjasmr::get_thumbnail_url_from_rj;

// Imports
use indicatif::{ProgressBar, ProgressStyle};
use id3::{Tag, TagLike, Version, frame::{Picture, PictureType}};


#[async_std::main]
async fn main() -> Result<()> {
    if env::args().len() == 1 {
        println!("Usage: rjasmr <link1> <link2> ...");
        std::process::exit(1);
    }

    for page_link in env::args().skip(1) {
        let image_data = match get_thumbnail_url_from_rj(&page_link).await {
            Ok(Some(url)) => {
                println!("Found thumbnail URL: {}", url);
                match download_image_data(&url).await {
                    Ok(data) => {
                        if data.len() < 1024 {
                            println!("Validation FAILED: Downloaded thumbnail is too small. Skipping.");
                            None
                        } else {
                            println!("Successfully downloaded and validated thumbnail data ({} bytes).", data.len());
                            Some(data)
                        }
                    },
                    Err(e) => {
                        println!("Failed to download thumbnail: {}", e);
                        None
                    }
                }
            },
            _ => {
                println!("Could not find a valid thumbnail for this page.");
                None
            }
        };

        let mut response = match get_audio_response(&page_link).await {
            Ok(res) => res,
            Err(e) => {
                println!("Error getting audio stream: {}", e);
                continue;
            }
        };

        let filename = match response.url().path_segments().and_then(|s| s.last()) {
            Some(name) => name.to_string(),
            None => {
                println!("Could not determine filename for URL: {}", response.url());
                continue;
            }
        };

        let total_size = response.content_length().unwrap_or(0);

        if total_size == 0 {
            println!("Error: Server returned Content-Length of 0. Cannot download file.");
            continue;
        }

        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));
        pb.set_message(format!("Downloading {}", &filename));

        let mut file = async_std::fs::File::create(&filename).await?;
        let mut downloaded: u64 = 0;

        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    file.write_all(&chunk).await?;
                    downloaded += chunk.len() as u64;
                    pb.set_position(downloaded);
                }
                Ok(None) => { break; }
                Err(e) => {
                    pb.abandon_with_message(format!("Error during download of {}: {}", &filename, e));
                    break;
                }
            }
        }

        pb.finish_with_message(format!("Finished download process for {}", &filename));

        if downloaded == total_size {
            println!("Verification successful: File size matches expected size.");
            if let Some(ref data) = image_data {
                if filename.ends_with(".mp3") {
                    println!("Embedding thumbnail into {}...", &filename);
                    if let Err(e) = embed_thumbnail(&filename, data) {
                        println!("Failed to embed thumbnail: {}", e);
                    } else {
                        println!("Successfully embedded thumbnail!");
                    }
                }
            } else {
                println!("No valid thumbnail data to embed.");
            }
        } else {
            println!("Verification FAILED: Downloaded size ({}) does not match expected size ({}). File is likely incomplete.", downloaded, total_size);
        }
    }
    Ok(())
}

async fn download_image_data(url: &str) -> std::result::Result<Vec<u8>, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.get(url)
    .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/110.0")
    .header("Accept", "image/avif,image/webp,*/*")
    .header("Referer", "https://japaneseasmr.com/")
    .send()
    .await?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

pub fn embed_thumbnail(file_path: &str, image_data: &[u8]) -> std::result::Result<(), id3::Error> {
    let mut tag = Tag::read_from_path(file_path).unwrap_or_else(|_| Tag::new());
    let mime_type = if image_data.starts_with(&[0xFF, 0xD8, 0xFF]) { "image/jpeg" }
    else if image_data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) { "image/png" }
    else { "image/unknown" };
    tag.add_frame(Picture {
        mime_type: mime_type.to_string(),
                  picture_type: PictureType::CoverFront,
                  description: String::from("Cover"),
                  data: image_data.to_vec(),
    });
    tag.write_to_path(file_path, Version::Id3v24)
}


pub async fn get_audio_response(page_url: &str) -> std::result::Result<reqwest::Response, Box<dyn Error>> {
    let body = reqwest::get(page_url).await?.text().await?;
    let document = Html::parse_document(&body);
    let selector = Selector::parse("video>source").unwrap();

    let audio_url = document.select(&selector)
    .filter_map(|e| e.value().attr("src"))
    .map(|s| s.to_string())
    .next()
    .ok_or("No audio source with a 'src' attribute found inside a <video> tag.")?;

    println!("Found audio link: {}", audio_url);

    let client = reqwest::Client::new();
    let response = client.get(&audio_url)
    .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/110.0")
    .header("Accept", "audio/webm,audio/ogg,audio/wav,audio/*;q=0.9,application/ogg;q=0.7,video/*;q=0.6,*/*;q=0.5")
    .header("Accept-Language", "en-US,en;q=0.5")
    .header("Range", "bytes=0-")
    .header("Connection", "keep-alive")
    .header("Referer", page_url)
    .header("Sec-Fetch-Dest", "audio")
    .header("Sec-Fetch-Mode", "no-cors")
    .header("Sec-Fetch-Site", "cross-site")
    .send()
    .await?;

    Ok(response)
}
