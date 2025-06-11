use scraper::{Html, Selector};
use std::io::Result;

/// Extract thumbnail URL from provided HTML body using the same logic as
/// `get_thumbnail_url_from_rj`.
pub fn get_thumbnail_url_from_html(body: &str) -> Option<String> {
    let document = Html::parse_document(body);
    document
        .select(&Selector::parse("#img_cover").unwrap())
        .next()
        .and_then(|e| e.value().attr("href"))
        .or_else(|| {
            document
                .select(&Selector::parse("#img_cover img").unwrap())
                .next()
                .and_then(|e| e.value().attr("data-src"))
        })
        .map(|s| s.to_string())
}

/// Fetch the given URL and attempt to extract the thumbnail URL from the page.
pub async fn get_thumbnail_url_from_rj(url: &str) -> Result<Option<String>> {
    let body = reqwest::get(url)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        .text()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(get_thumbnail_url_from_html(&body))
}
