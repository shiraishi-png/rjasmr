use rjasmr::get_thumbnail_url_from_html;

#[async_std::test]
async fn thumbnail_from_anchor_href() {
    let html = r#"
        <html><body>
        <a id=\"img_cover\" href=\"https://example.com/thumb.jpg\">cover</a>
        </body></html>
    "#;
    let url = get_thumbnail_url_from_html(html);
    assert_eq!(url.as_deref(), Some("https://example.com/thumb.jpg"));
}

#[async_std::test]
async fn thumbnail_from_img_data_src() {
    let html = r#"
        <html><body>
        <div id=\"img_cover\"><img data-src=\"https://example.com/img.jpg\" /></div>
        </body></html>
    "#;
    let url = get_thumbnail_url_from_html(html);
    assert_eq!(url.as_deref(), Some("https://example.com/img.jpg"));
}
