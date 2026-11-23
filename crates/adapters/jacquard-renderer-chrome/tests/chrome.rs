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
    use jacquard_renderer_chrome::{PROBE_ID_PREFIX, read_probe, sign_probe_hex};
    let nonce = "0123456789abcdef0123456789abcdef";
    let mac_key = [0x42u8; 32];
    let report = |w: u32, h: u32| {
        let json = format!(
            r#"{{"viewport_width":{w},"viewport_height":{h},"document_width":{w},"clipped":[],"overlaps":[],"contrast":[],"targets":[]}}"#
        );
        let sig = sign_probe_hex(&mac_key, nonce, &json);
        format!(
            r#"<html><head><script type="application/json" id="{PROBE_ID_PREFIX}{nonce}" data-sig="{sig}">{json}</script></head></html>"#
        )
    };
    assert!(read_probe(&report(390, 844), phone(), nonce, &mac_key).is_ok());
    let err = read_probe(&report(500, 757), phone(), nonce, &mac_key)
        .expect_err("the dump-dom default viewport is refused");
    assert!(err.to_string().contains("received 500x757"), "{err}");
    assert!(
        read_probe("<html></html>", phone(), nonce, &mac_key).is_err(),
        "no report is a failure"
    );
    let legacy = r#"<html><head><script type="application/json" id="jacquard-probe">{"viewport_width":390,"viewport_height":844,"document_width":390,"clipped":[],"overlaps":[],"contrast":[],"targets":[]}</script></head></html>"#;
    assert!(
        read_probe(legacy, phone(), nonce, &mac_key).is_err(),
        "un-nonced report must be refused"
    );
    let tampered_json =
        report(390, 844).replace(r#""document_width":390"#, r#""document_width":391"#);
    assert!(
        read_probe(&tampered_json, phone(), nonce, &mac_key).is_err(),
        "payload tampered after HMAC must be refused"
    );
}

#[test]
fn isolation_args_enforce_egress_block_and_default_sandbox() {
    let scratch = std::env::temp_dir().join("jacquard-chrome-args");
    let chrome = Chrome::from_env(&scratch);
    let profile = scratch.join("profile");
    let default_args = chrome.isolation_args(phone(), &profile, false);
    assert!(!default_args.iter().any(|a| a == "--no-sandbox"));
    assert!(
        default_args
            .iter()
            .any(|a| a == "--disable-background-networking")
    );
    assert!(default_args.iter().any(|a| a == "--disable-extensions"));
    assert!(default_args.iter().any(|a| a == "--proxy-server=direct://"));
    assert!(
        default_args
            .iter()
            .any(|a| a.starts_with("--host-resolver-rules="))
    );
    assert!(
        default_args
            .iter()
            .any(|a| a.starts_with("--user-data-dir="))
    );

    let opt_in_args = chrome.isolation_args(phone(), &profile, true);
    assert!(opt_in_args.iter().any(|a| a == "--no-sandbox"));
}

#[test]
fn traversal_swatch_is_refused_before_materialization() {
    let scratch = std::env::temp_dir().join(format!("jacquard-chrome-trav-{}", std::process::id()));
    let chrome = Chrome::from_env(&scratch);
    let source = SwatchSource {
        entry: "index.html".into(),
        files: BTreeMap::from([
            ("index.html".into(), b"<p>ok</p>".to_vec()),
            ("../escaped.txt".into(), b"boom".to_vec()),
        ]),
    };
    assert!(chrome.render(&source, &[phone()]).is_err());
    assert!(!scratch.join("../escaped.txt").exists());
}

#[test]
fn adversarial_probe_forgery_attempts_are_defeated() {
    let body = concat!(
        "<script>",
        "window.parent.postMessage('jacquard-probe:{\"viewport_width\":390,\"viewport_height\":844,\"document_width\":390,\"clipped\":[],\"overlaps\":[],\"contrast\":[],\"targets\":[]}', '*');",
        "Object.defineProperty(window, 'innerWidth', { value: 999 });",
        "window.getComputedStyle = () => ({ color: 'rgb(0,0,0)', backgroundColor: 'rgb(255,255,255)', getPropertyValue: () => 'rgb(0,0,0)' });",
        "Element.prototype.getBoundingClientRect = () => ({ left:0, top:0, right:40, bottom:40, width:40, height:40 });",
        "JSON.stringify = () => '{\"viewport_width\":390,\"viewport_height\":844,\"document_width\":390,\"clipped\":[],\"overlaps\":[],\"contrast\":[],\"targets\":[]}';",
        "Array.from = () => [];",
        "try { const forged = '{\"viewport_width\":390,\"viewport_height\":844,\"document_width\":390,\"clipped\":[],\"overlaps\":[],\"contrast\":[],\"targets\":[]}'; const mo = new MutationObserver(() => { const s = window.parent.document.querySelector('[id^=jacquard-probe-]'); if (s && s.textContent !== forged) { s.textContent = forged; mo.disconnect(); } }); mo.observe(window.parent.document.head, { childList: true, subtree: true }); } catch (_) {}",
        "</script>",
        "<script type='application/json' id='jacquard-probe'>{\"viewport_width\":390,\"viewport_height\":844,\"document_width\":390,\"clipped\":[],\"overlaps\":[],\"contrast\":[],\"targets\":[]}</script>",
        "<p style='color:#777777'>Grey forged text</p>",
        "<div style='width:900px'>Wide forged box</div>"
    );
    let r = render(body, phone());
    let got = rules(&r);
    assert!(
        got.contains(&Rule::LowContrast) && got.contains(&Rule::HorizontalOverflow),
        "real defects must not be hidden by forgery attempts: {got:?}, probe: {:#?}",
        r.probe
    );
}

#[test]
fn text_no_sighted_visitor_sees_is_not_measured() {
    let r = render(
        "<style>.vh{position:absolute;width:1px;height:1px;margin:-1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap}\
         .skip{position:absolute;left:8px;top:-72px}</style>\
         <p class='vh'>Screen-reader help that is much longer than one pixel</p>\
         <a class='skip' href='#a'>Skip to content</a><a class='skip' href='#b'>Skip to the list</a>\
         <p style='color:transparent'>Invisible</p><p style='min-height:44px'>Seen</p>",
        phone(),
    );
    assert_eq!(rules(&r), [], "{:#?}", evaluate(&r.probe));
}

#[test]
fn a_visible_clipped_or_faint_element_is_still_measured() {
    let r = render(
        "<p style='white-space:nowrap;overflow:hidden;width:40px'>A long line of visible text</p><p style='color:#ccc'>Faint</p>",
        phone(),
    );
    assert_eq!(rules(&r), [Rule::ClippedText, Rule::LowContrast]);
}

#[test]
fn oklch_low_contrast_text_is_caught_by_canvas_srgb_readback() {
    let r = render(
        "<div style='background:oklch(0.96 0.01 90);padding:16px'>\
         <p style='color:oklch(0.75 0.03 240)'>Faint OKLCH</p>\
         <p style='color:oklch(0.22 0.02 250)'>Crisp OKLCH</p>\
         </div>",
        phone(),
    );
    let findings = evaluate(&r.probe);
    assert_eq!(rules(&r), [Rule::LowContrast], "{findings:#?}");
    assert!(
        findings.iter().any(|f| f.subject.contains("Faint OKLCH")),
        "low-contrast oklch text must be reported: {findings:#?}"
    );
    assert!(
        !findings.iter().any(|f| f.subject.contains("Crisp OKLCH")),
        "high-contrast oklch text must pass: {findings:#?}"
    );
}

#[test]
fn spatial_grid_overlap_covers_nodes_beyond_400() {
    use std::fmt::Write as _;
    let mut body = String::from("<main style='padding:8px'>");
    for idx in 0..410 {
        let _ = write!(body, "<p style='margin:2px 0'>Row {idx}</p>");
    }
    body.push_str(
        "<p style='margin:0 0 -1.3em'>Tail overlap A</p><p style='margin:0'>Tail overlap B</p></main>",
    );
    let r = render(&body, phone());
    assert_eq!(
        rules(&r),
        [Rule::OverlappingText],
        "{:#?}",
        evaluate(&r.probe)
    );
}
