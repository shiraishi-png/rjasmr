use rjasmr::get_audio_url_from_html;

#[async_std::test]
async fn audio_from_video_sources() {
    let html = r#"
        <div id="cleanp_audio" class="cleanPlayer" theme="dark">
        <video title="Track 1" descr="Track 1" preload="metadata" poster="https://pic.weeabo0.xyz/RJ284161_img_main.jpg">
        <source src="https://v.weeab0o.xyz/RJ284161.mp3" type="audio/mpeg"/>
        <source src="https://v.weeab0o.xyz/RJ284161.m4a" type="audio/mp4"/>
        </video>
        </div>
    "#;
    let url = get_audio_url_from_html(html);
    assert_eq!(url.as_deref(), Some("https://v.weeab0o.xyz/RJ284161.mp3"));
}
