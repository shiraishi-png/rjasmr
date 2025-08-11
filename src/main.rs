use async_std::io::WriteExt;
use futures::{stream, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use scraper::{Html, Selector};
use std::env;
use std::error::Error;
use std::process::Command;
use std::sync::Arc;

// Tune this
const CONCURRENCY: usize = 2;

#[async_std::main]
async fn main() -> std::io::Result<()> {
    let pages: Vec<String> = env::args().skip(1).collect();
    if pages.is_empty() {
        eprintln!("Usage: rjasmr <page_url1> <page_url2> ...");
        std::process::exit(1);
    }

    let mp = Arc::new(MultiProgress::new());

    stream::iter(pages)
    .map(|page_link| {
        let mp = mp.clone();
        async move {
            if let Err(e) = process_page(page_link.clone(), mp.clone()).await {
                let _ = mp.println(format!("[error] {e}"));
            }
        }
    })
    .buffer_unordered(CONCURRENCY)
    .for_each(|_| async {})
    .await;

    // let bars finish rendering
    let _ = mp.clear();
    Ok(())
}

async fn process_page(page_link: String, mp: Arc<MultiProgress>) -> std::io::Result<()> {
    let image_data = match get_cover_url(&page_link).await {
        Ok(Some(url)) => download_image_data(&url, &page_link).await.ok(),
        _ => None,
    };

    let media_url = match extract_media_url(&page_link).await {
        Ok(u) => u,
        Err(e) => {
            let _ = mp.println(format!("No media URL on page {page_link}: {e}"));
            return Ok(());
        }
    };

    if media_url.ends_with(".m3u8") {
        if let Err(e) = hls_to_mp3_with_optional_cover(&media_url, &page_link, image_data.as_deref(), &mp) {
            let _ = mp.println(format!("ffmpeg HLS → MP3 failed ({media_url}): {e}"));
        }
        return Ok(());
    }

    match download_file(&media_url, &page_link, &mp).await {
        Ok(saved) => {
            if saved.to_ascii_lowercase().ends_with(".mp3") {
                if let Some(ref data) = image_data {
                    if let Err(e) = attach_cover_to_existing_mp3(&saved, data, &mp) {
                        let _ = mp.println(format!("Attach cover failed: {e}"));
                    }
                }
            }
            let _ = mp.println(format!("Saved {saved}"));
        }
        Err(e) => {
            let _ = mp.println(format!("Download failed ({media_url}): {e}"));
        }
    }

    Ok(())
}

// ---------- page parsing ----------
async fn extract_media_url(page_url: &str) -> Result<String, Box<dyn Error>> {
    let body = reqwest::Client::new()
    .get(page_url)
    .header("User-Agent", "Mozilla/5.0")
    .send()
    .await?
    .text()
    .await?;

    let doc = Html::parse_document(&body);

    if let Some(u) = doc
        .select(&Selector::parse("video > source[src]").unwrap())
        .next()
        .and_then(|e| e.value().attr("src"))
        {
            return Ok(u.to_string());
        }
        if let Some(u) = doc
            .select(&Selector::parse("audio#audio").unwrap())
            .next()
            .and_then(|e| e.value().attr("src"))
            {
                return Ok(u.to_string());
            }

            Err("no <video><source src> or <audio id=\"audio\" src> found".into())
}

async fn get_cover_url(url: &str) -> std::io::Result<Option<String>> {
    let body = reqwest::Client::new()
    .get(url)
    .header("User-Agent", "Mozilla/5.0")
    .send()
    .await
    .map_err(to_ioe)?
    .text()
    .await
    .map_err(to_ioe)?;
    let doc = Html::parse_document(&body);
    Ok(find_cover_url(&doc))
}

fn find_cover_url(doc: &Html) -> Option<String> {
    if let Ok(sel) = Selector::parse("#img_cover[href]") {
        if let Some(a) = doc.select(&sel).next() {
            if let Some(href) = a.value().attr("href") {
                return Some(href.to_string());
            }
        }
    }
    if let Ok(sel) = Selector::parse("#img_cover img") {
        if let Some(img) = doc.select(&sel).next() {
            if let Some(ds) = img.value().attr("data-src") {
                return Some(ds.to_string());
            }
            if let Some(src) = img.value().attr("src") {
                return Some(src.to_string());
            }
        }
    }
    if let Ok(sel) = Selector::parse("video[poster]") {
        if let Some(v) = doc.select(&sel).next() {
            if let Some(poster) = v.value().attr("poster") {
                return Some(poster.to_string());
            }
        }
    }
    if let Ok(sel) = Selector::parse("meta[property='og:image']") {
        if let Some(m) = doc.select(&sel).next() {
            if let Some(content) = m.value().attr("content") {
                return Some(content.to_string());
            }
        }
    }
    None
}

// ---------- downloads ----------
async fn download_image_data(url: &str, referer: &str) -> Result<Vec<u8>, reqwest::Error> {
    let bytes = reqwest::Client::new()
    .get(url)
    .header("User-Agent", "Mozilla/5.0")
    .header("Referer", referer)
    .header("Accept", "image/jpeg,*/*;q=0.8")
    .send()
    .await?
    .bytes()
    .await?;
    Ok(bytes.to_vec())
}

async fn download_file(media_url: &str, referer: &str, mp: &Arc<MultiProgress>) -> std::io::Result<String> {
    let client = reqwest::Client::new();
    let mut resp = client
    .get(media_url)
    .header("User-Agent", "Mozilla/5.0")
    .header("Referer", referer)
    .send()
    .await
    .map_err(to_ioe)?;

    let fname = resp
    .url()
    .path_segments()
    .and_then(|s| s.last())
    .filter(|s| !s.is_empty())
    .unwrap_or("audio.bin")
    .to_string();

    let total = resp.content_length();
    let pb = if let Some(sz) = total {
        let pb = ProgressBar::new(sz);
        pb.set_style(
            ProgressStyle::default_bar()
            .template("{msg}\n{spinner} [{elapsed_precise}] [{bar:40}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .unwrap()
            .progress_chars("#>-"),
        );
        pb.set_message(format!("Downloading {fname}"));
        mp.add(pb)
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_message(format!("Downloading {fname}"));
        mp.add(pb)
    };

    let mut file = async_std::fs::File::create(&fname).await?;
    let mut downloaded: u64 = 0;

    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                file.write_all(&chunk).await?;
                downloaded += chunk.len() as u64;
                if let Some(_) = total {
                    pb.set_position(downloaded);
                } else {
                    pb.tick();
                }
            }
            Ok(None) => break,
            Err(e) => {
                pb.finish_with_message(format!("Download error: {e}"));
                return Err(to_ioe(e));
            }
        }
    }

    pb.finish_with_message(format!("Saved {fname}"));
    Ok(fname)
}

// ---------- ffmpeg paths ----------
fn hls_to_mp3_with_optional_cover(
    hls_url: &str,
    referer: &str,
    cover_bytes: Option<&[u8]>,
    mp: &Arc<MultiProgress>,
) -> std::io::Result<()> {
    let stem = hls_url
    .rsplit('/')
    .next()
    .unwrap_or("audio")
    .trim_end_matches(".m3u8");
    let out = format!("{stem}.mp3");

    let mut args: Vec<String> = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "warning".into(),
        "-nostats".into(),
        "-y".into(),
        "-headers".into(),
        format!("Referer: {referer}\r\nUser-Agent: Mozilla/5.0\r\n"),
            "-i".into(),
            hls_url.into(),
    ];

    let _cover_guard;
    if let Some(bytes) = cover_bytes {
        let cover = ensure_jpeg_file(bytes)?;
        let path = cover.path.to_string_lossy().into_owned();
        _cover_guard = Some(cover);

        args.extend([
            "-i".into(),
                    path,
                    "-map".into(),
                    "0:a".into(),
                    "-map".into(),
                    "1".into(),
                    "-c:a".into(),
                    "libmp3lame".into(),
                    "-b:a".into(),
                    "192k".into(),
                    "-c:v".into(),
                    "mjpeg".into(),
                    "-disposition:v".into(),
                    "attached_pic".into(),
                    "-metadata:s:v".into(),
                    "title=Album cover".into(),
                    "-metadata:s:v".into(),
                    "comment=Cover (front)".into(),
                    "-id3v2_version".into(),
                    "3".into(),
                    out.clone(),
        ]);
    } else {
        _cover_guard = None;
        args.extend([
            "-vn".into(),
                    "-c:a".into(),
                    "libmp3lame".into(),
                    "-b:a".into(),
                    "192k".into(),
                    "-id3v2_version".into(),
                    "3".into(),
                    out.clone(),
        ]);
    }

    let pb = mp.add(ProgressBar::new_spinner());
    pb.set_message(format!("FFmpeg {}", out));
    let status = Command::new("ffmpeg").args(args).status()?;
    if status.success() {
        pb.finish_with_message(format!("Saved {out}"));
        Ok(())
    } else {
        pb.finish_with_message("ffmpeg failed");
        Err(ioerr("ffmpeg failed"))
    }
}

fn attach_cover_to_existing_mp3(infile: &str, cover_bytes: &[u8], mp: &Arc<MultiProgress>) -> std::io::Result<()> {
    let cover = ensure_jpeg_file(cover_bytes)?;
    let outfile = format!("{infile}.tmp.mp3");

    let pb = mp.add(ProgressBar::new_spinner());
    pb.set_message(format!("Attaching cover → {infile}"));

    let status = Command::new("ffmpeg")
    .args([
        "-hide_banner",
        "-loglevel",
        "warning",
        "-nostats",
        "-y",
        "-i",
        infile,
        "-i",
        cover.path.to_str().unwrap(),
          "-map",
          "0:a",
          "-map",
          "1",
          "-c:a",
          "copy",
          "-c:v",
          "mjpeg",
          "-disposition:v",
          "attached_pic",
          "-metadata:s:v",
          "title=Album cover",
          "-metadata:s:v",
          "comment=Cover (front)",
          "-id3v2_version",
          "3",
          &outfile,
    ])
    .status()?;

    if !status.success() {
        pb.finish_with_message("Attach cover failed");
        return Err(ioerr("ffmpeg attach failed"));
    }
    std::fs::rename(outfile, infile)?;
    pb.finish_with_message(format!("Cover attached → {infile}"));
    Ok(())
}

// ---------- cover helpers ----------
struct CoverTmp {
    _dir: tempfile::TempDir,
    path: std::path::PathBuf,
}

fn ensure_jpeg_file(bytes: &[u8]) -> std::io::Result<CoverTmp> {
    use std::io::Write as IoWrite;

    let tdir = tempfile::tempdir()?;
    let src = tdir.path().join("src.bin");
    let dst = tdir.path().join("cover.jpg");

    std::fs::File::create(&src)?.write_all(bytes)?;

    if looks_like_jpeg(bytes) {
        std::fs::copy(&src, &dst)?;
    } else {
        let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            src.to_str().unwrap(),
              "-frames:v",
              "1",
              "-f",
              "image2",
              "-update",
              "1",
              dst.to_str().unwrap(),
        ])
        .status()?;
        if !status.success() || !dst.exists() {
            return Err(ioerr("cover convert failed"));
        }
    }

    Ok(CoverTmp { _dir: tdir, path: dst })
}

fn looks_like_jpeg(b: &[u8]) -> bool {
    b.len() > 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF
}

// ---------- misc helpers ----------
fn ioerr(msg: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, msg)
}

fn to_ioe<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
}
