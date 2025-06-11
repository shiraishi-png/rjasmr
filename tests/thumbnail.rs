use rjasmr::get_thumbnail_url_from_html;

#[async_std::test]
async fn thumbnail_from_anchor_href() {
    let html = r#"
        <html><body>
        <a id="img_cover" href="https://example.com/thumb.jpg">cover</a>
        </body></html>
    "#;
    let url = get_thumbnail_url_from_html(html);
    assert_eq!(url.as_deref(), Some("https://example.com/thumb.jpg"));
}

#[async_std::test]
async fn thumbnail_from_img_data_src() {
    let html = r#"
        <html><body>
        <div id="img_cover"><img data-src="https://example.com/img.jpg" /></div>
        </body></html>
    "#;
    let url = get_thumbnail_url_from_html(html);
    assert_eq!(url.as_deref(), Some("https://example.com/img.jpg"));
}

#[async_std::test]
async fn thumbnail_from_img_content() {
    let html = r#"
        <div class="img-content img-display-container">
        <img class="imgSlides lazyload lazy" src="data:image/svg+xml,%3Csvg%20xmlns='http://www.w3.org/2000/svg'%20viewBox='0%200%201%201'%3E%3C/svg%3E" data-src="https://pic.weeabo0.xyz/RJ284161_img_main.jpg"  style="width:100%" />
        <img class="imgSlides lazyload lazy" src="data:image/svg+xml,%3Csvg%20xmlns='http://www.w3.org/2000/svg'%20viewBox='0%200%201%201'%3E%3C/svg%3E" data-src="https://pic.weeabo0.xyz/RJ284161(1).jpg"  style="width:100%">
        </div>
    "#;
    let url = get_thumbnail_url_from_html(html);
    assert_eq!(url.as_deref(), Some("https://pic.weeabo0.xyz/RJ284161_img_main.jpg"));
}
