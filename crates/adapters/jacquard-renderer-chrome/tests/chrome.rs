//! The renderer against real Chrome. Known answers come from outside the
//! code: black on white is 21:1 and `#777777` on white is 4.48:1 (the WCAG
//! relative-luminance formula, as the `WebAIM` contrast checker reports it).
//! Chrome must be present; a missing browser fails the test rather than
//! skipping it.

use std::collections::BTreeMap;

use jacquard_core::probe::{Rule, evaluate};
use jacquard_core::viewport::Viewport;
use jacquard_ports::{Render, Renderer, SwatchSource};
use jacquard_renderer_chrome::Chrome;

fn render(body: &str, viewport: Viewport) -> Render {
    let html = format!(
        "<!doctype html><html><head><meta charset=utf-8><meta name=viewport content='width=device-width'>\
         <style>body{{margin:0;font:16px/1.5 sans-serif;background:#fff;color:#000}}</style></head><body>{body}</body></html>"
    );
    let source = SwatchSource {
        entry: "index.html".into(),
        files: BTreeMap::from([("index.html".to_string(), html.into_bytes())]),
    };
    let scratch = std::env::temp_dir().join(format!("jacquard-chrome-test-{}", std::process::id()));
    let chrome = Chrome::from_env(&scratch);
    assert!(
        chrome.binary().is_file(),
        "Chrome is required: set JACQUARD_CHROME (looked at {})",
        chrome.binary().display()
    );
    let mut renders = chrome.render(&source, &[viewport]).expect("rendered");
    renders.pop().expect("one render")
}

fn phone() -> Viewport {
    Viewport {
        width: 390,
        height: 844,
    }
}

fn rules(r: &Render) -> Vec<Rule> {
    let mut v: Vec<Rule> = evaluate(&r.probe).into_iter().map(|f| f.rule).collect();
    v.dedup();
    v
}

#[test]
fn a_plain_page_renders_clean_with_a_png() {
    let r = render(
        "<main style='padding:16px'><h1>Hello</h1><p>Plain text.</p><button style='min-height:44px;padding:8px 16px'>Go</button></main>",
        phone(),
    );
    assert!(r.png.starts_with(b"\x89PNG"), "a PNG screenshot");
    assert_eq!(rules(&r), [], "{:#?}", evaluate(&r.probe));
    let black = r
        .probe
        .contrast
        .iter()
        .find(|c| c.element.starts_with("h1"))
        .expect("h1 measured");
    assert!(
        (black.ratio - 21.0).abs() < 0.01,
        "black on white is 21:1, got {}",
        black.ratio
    );
    assert!(black.large, "a default h1 is large text");
}

#[test]
fn contrast_matches_the_wcag_formula_at_the_boundary() {
    let r = render(
        "<p style='color:#777777'>Grey text</p><p style='color:#767676'>Darker grey</p>",
        phone(),
    );
    let ratio = |needle: &str| {
        r.probe
            .contrast
            .iter()
            .find(|c| c.element.contains(needle))
            .expect("measured")
            .ratio
    };
    assert!(
        (ratio("Grey text") - 4.48).abs() < 0.01,
        "#777 is 4.48:1, got {}",
        ratio("Grey text")
    );
    assert!(
        (ratio("Darker grey") - 4.54).abs() < 0.01,
        "#767676 is 4.54:1, got {}",
        ratio("Darker grey")
    );
    assert_eq!(rules(&r), [Rule::LowContrast], "only #777 fails");
}

#[test]
fn each_defect_trips_its_own_rule() {
    let cases = [
        (
            "<h1 style='white-space:nowrap;overflow:hidden;width:3em'>A long heading</h1>",
            Rule::ClippedText,
        ),
        (
            "<div style='width:900px'>Wide</div>",
            Rule::HorizontalOverflow,
        ),
        (
            "<h1 style='margin:0 0 -1.2em'>Heading</h1><p style='margin:0'>Paragraph under it</p>",
            Rule::OverlappingText,
        ),
        (
            "<button style='font-size:10px;padding:0;line-height:12px'>Go</button>",
            Rule::SmallTarget,
        ),
    ];
    for (body, rule) in cases {
        let r = render(body, phone());
        assert_eq!(rules(&r), [rule], "{body}: {:#?}", evaluate(&r.probe));
    }
}

#[test]
fn inline_links_in_a_sentence_are_exempt_from_target_size() {
    let r = render("<p>Read <a href='#x'>this</a> first.</p>", phone());
    assert_eq!(rules(&r), []);
    assert!(
        r.probe.targets.iter().any(|t| t.inline),
        "the link was measured as inline"
    );
}

#[test]
fn a_missing_entry_is_a_render_failure() {
    let scratch =
        std::env::temp_dir().join(format!("jacquard-chrome-missing-{}", std::process::id()));
    let chrome = Chrome::from_env(&scratch);
    let source = SwatchSource {
        entry: "index.html".into(),
        files: BTreeMap::new(),
    };
    assert!(chrome.render(&source, &[phone()]).is_err());
}

#[test]
fn a_page_that_received_another_viewport_is_refused() {
    use jacquard_renderer_chrome::read_probe;
    let report = |w: u32, h: u32| {
        format!(
            r#"<html><head><script type="application/json" id="jacquard-probe">{{"viewport_width":{w},"viewport_height":{h},"document_width":{w},"clipped":[],"overlaps":[],"contrast":[],"targets":[]}}</script></head></html>"#
        )
    };
    assert!(read_probe(&report(390, 844), phone()).is_ok());
    let err = read_probe(&report(500, 757), phone())
        .expect_err("the dump-dom default viewport is refused");
    assert!(err.to_string().contains("received 500x757"), "{err}");
    assert!(
        read_probe("<html></html>", phone()).is_err(),
        "no report is a failure"
    );
}
