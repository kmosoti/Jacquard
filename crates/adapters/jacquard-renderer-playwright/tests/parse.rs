//! The driver's output: a render needs a screenshot and a probe report for
//! the viewport asked for; a driver error, a missing part or another
//! viewport is a failure.

use jacquard_renderer_playwright::parse_line;

fn line(w: u32, h: u32, png: Option<&str>, error: Option<&str>) -> String {
    serde_json::json!({
        "url": "http://x/",
        "viewport": [390, 844],
        "png": png,
        "error": error,
        "probe": {"viewport_width": w, "viewport_height": h, "document_width": w, "clipped": ["h1"], "overlaps": [], "contrast": [], "targets": []},
    })
    .to_string()
}

#[test]
fn a_complete_line_is_a_render() {
    let png = std::env::temp_dir().join(format!("jacquard-pw-parse-{}.png", std::process::id()));
    std::fs::write(&png, b"\x89PNG").expect("write");
    let r = parse_line(&line(390, 844, png.to_str(), None), "http://x/").expect("render");
    assert_eq!(r.png, b"\x89PNG");
    assert_eq!(r.probe.clipped, ["h1"]);
    assert_eq!(r.viewport.to_string(), "390x844");
}

#[test]
fn another_viewport_a_driver_error_or_a_missing_screenshot_fails() {
    assert!(
        parse_line(&line(500, 757, Some("/nonexistent.png"), None), "u")
            .unwrap_err()
            .to_string()
            .contains("received 500x757")
    );
    assert!(
        parse_line(
            &line(390, 844, None, Some("net::ERR_CONNECTION_REFUSED")),
            "u"
        )
        .unwrap_err()
        .to_string()
        .contains("REFUSED")
    );
    assert!(parse_line(&line(390, 844, None, None), "u").is_err());
    assert!(parse_line("not json", "u").is_err());
}
