//! Adversarial probe forgery and wrapper isolation tests against real Chrome.

use std::collections::BTreeMap;

use jacquard_core::probe::{Rule, evaluate};
use jacquard_core::viewport::Viewport;
use jacquard_ports::{Failure, Render, Renderer, SwatchSource};
use jacquard_renderer_chrome::Chrome;

fn phone() -> Viewport {
    Viewport {
        width: 390,
        height: 844,
    }
}

fn render_raw(html: String, vp: Viewport) -> Result<Render, Failure> {
    let source = SwatchSource {
        entry: "index.html".into(),
        files: BTreeMap::from([("index.html".to_string(), html.into_bytes())]),
    };
    let scratch = std::env::temp_dir().join(format!("jacquard-chrome-sec-{}", std::process::id()));
    let chrome = Chrome::from_env(&scratch);
    assert!(
        chrome.binary().is_file(),
        "Chrome is required: set JACQUARD_CHROME (looked at {})",
        chrome.binary().display()
    );
    chrome
        .render(&source, &[vp])
        .map(|mut v| v.pop().expect("one render"))
}

fn rules(r: &Render) -> Vec<Rule> {
    let mut v: Vec<Rule> = evaluate(&r.probe).into_iter().map(|f| f.rule).collect();
    v.dedup();
    v
}

#[test]
fn parent_head_script_injection_bypass_is_defeated() {
    // Reproduces the parent-realm inline `<script>` injection exploit against `--dump-dom`.
    let html = concat!(
        "<!doctype html><html><head><meta charset=utf-8><meta name=viewport content='width=device-width'>",
        "<style>body{margin:0;background:#fff;color:#000}h1{width:120px;white-space:nowrap;overflow:hidden;margin-left:-900px}</style>",
        "</head><body>",
        "<h1>A very long clipped heading pushed off screen</h1>",
        "<a href='#x' style='display:inline-block;width:10px;height:10px'>x</a>",
        "<p style='color:#d9d9d9'>Low contrast text</p>",
        "<script>",
        "const P = window.parent;",
        "const forged = JSON.stringify({viewport_width:1440,viewport_height:900,document_width:1440,clipped:[],overlaps:[],contrast:[],targets:[]});",
        "const s = P.document.createElement('script');",
        "s.textContent = \"new MutationObserver((ms,o)=>{const n=document.querySelector('[id^=jacquard-probe-]');if(n){n.textContent='\" + forged + \"';o.disconnect();}}).observe(document.head,{childList:true,subtree:true});\";",
        "P.document.head.appendChild(s);",
        "</script></body></html>"
    )
    .to_string();

    let vp = Viewport {
        width: 1440,
        height: 900,
    };
    let render = render_raw(html, vp).expect("CSP blocks injected script cleanly");
    let got = rules(&render);
    assert!(
        got.contains(&Rule::LowContrast) && got.contains(&Rule::SmallTarget),
        "injected parent head script must not forge a clean render: {got:?}, probe: {:#?}",
        render.probe
    );
}

#[test]
fn sibling_iframe_and_parent_mutation_attacks_cannot_forge_clean_render() {
    // 1. Sibling iframe injected into parent.body & parent.head trying to overwrite probe DOM.
    let sibling_html = concat!(
        "<!doctype html><html><head><meta charset=utf-8></head><body style='margin:0;background:#fff'>",
        "<p style='color:#777777'>Low contrast</p><div style='width:900px'>Wide</div>",
        "<script>",
        "if (window.location.search !== '?child=1') {",
        "  for (const host of [window.parent.document.body, window.parent.document.head]) {",
        "    try { const f = window.parent.document.createElement('iframe'); f.src = '/index.html?child=1'; host.appendChild(f); } catch (_) {}",
        "  }",
        "} else {",
        "  try { const mo = new window.parent.MutationObserver(() => { const n = window.parent.document.querySelector('[id^=jacquard-probe-]'); if (n) n.textContent = JSON.stringify({viewport_width:390,viewport_height:844,document_width:390,clipped:[],overlaps:[],contrast:[],targets:[]}); }); mo.observe(window.parent.document.head, {childList:true,subtree:true}); } catch (_) {}",
        "}",
        "</script></body></html>"
    ).to_string();
    if let Ok(r) = render_raw(sibling_html, phone()) {
        let got = rules(&r);
        assert!(
            got.contains(&Rule::LowContrast) && got.contains(&Rule::HorizontalOverflow),
            "sibling iframe tampering must not return clean: {got:?}"
        );
    }

    // 2. Direct cross-frame MutationObserver mutating probe textContent after emit() fails HMAC.
    let direct_mo_html = concat!(
        "<!doctype html><html><head><meta charset=utf-8></head><body style='margin:0;background:#fff'>",
        "<p style='color:#777777'>Low contrast</p>",
        "<script>",
        "try { window.parent.Number = () => 0; window.parent.Boolean = () => false; } catch (_) {}",
        "const forged = JSON.stringify({viewport_width:390,viewport_height:844,document_width:390,clipped:[],overlaps:[],contrast:[],targets:[]});",
        "try { const mo = new window.parent.MutationObserver(() => { const n = window.parent.document.querySelector('[id^=jacquard-probe-]'); if (n && n.textContent !== forged) { n.textContent = forged; } }); mo.observe(window.parent.document.head, {childList:true,subtree:true}); } catch (_) {}",
        "</script></body></html>"
    ).to_string();
    match render_raw(direct_mo_html, phone()) {
        Ok(r) => {
            let got = rules(&r);
            assert!(
                got.contains(&Rule::LowContrast),
                "parent Number/Boolean/MutationObserver spoof must never return clean: {got:?}"
            );
        }
        Err(e) => {
            assert!(
                e.to_string().contains("HMAC") || e.to_string().contains("probe"),
                "tampered probe must fail verification explicitly: {e}"
            );
        }
    }
}

#[test]
fn wrapper_refusal_of_refetch_and_script_tag_leakage() {
    // Candidate tries to inspect parent <script> tag and re-fetch wrapper HTML.
    let html = concat!(
        "<!doctype html><html><head><meta charset=utf-8></head><body style='margin:0;background:#fff'>",
        "<p style='color:#777777'>Grey</p>",
        "<script>",
        "const scriptsInParent = window.parent.document.querySelectorAll('script').length;",
        "fetch(window.parent.location.href).then(r => {",
        "  if (r.ok || scriptsInParent > 0) {",
        "    document.body.innerHTML = '<p style=\"color:#000\">Clean after leak</p>';",
        "  }",
        "}).catch(() => {});",
        "</script></body></html>"
    ).to_string();
    let r = render_raw(html, phone()).expect("renders cleanly");
    assert_eq!(rules(&r), [Rule::LowContrast]);
}

#[test]
fn candidate_prototype_poisoning_and_cloaking_scripts_are_blocked() {
    let cases = [
        (
            "NodeList iterator hijack",
            "<script>NodeList.prototype[Symbol.iterator]=function*(){};</script>",
        ),
        (
            "DOMRectList length hijack",
            "<script>Object.defineProperty(DOMRectList.prototype,\"length\",{get(){return 0}});</script>",
        ),
        (
            "DOMRect width/height hijack",
            "<script>for(const k of [\"width\",\"height\"])Object.defineProperty(DOMRect.prototype,k,{get(){return 48}});</script>",
        ),
        (
            "probe-url cloaking",
            "<style>.probe p{color:#000}</style><script>if(window.parent.location.pathname.indexOf(\"__jacquard_probe-\")>=0)document.documentElement.className=\"probe\";</script>",
        ),
    ];

    for (label, attack) in cases {
        let html = format!(
            "<!doctype html><html><head><meta charset=utf-8><meta name=viewport content='width=device-width'>\
             {attack}\
             <style>body{{margin:0;background:#fff;color:#000}}h1{{width:60px;white-space:nowrap;overflow:hidden}}</style></head>\
             <body><h1>Clipped heading text that exceeds sixty pixels</h1>\
             <p style='color:#eeeeee'>Faint text</p>\
             <a href='#x' style='display:inline-block;width:10px;height:10px'>x</a>\
             </body></html>"
        );
        let r = render_raw(
            html,
            Viewport {
                width: 1440,
                height: 900,
            },
        )
        .unwrap_or_else(|e| panic!("{label}: render failed unexpectedly: {e}"));
        let got = rules(&r);
        assert!(
            got.contains(&Rule::ClippedText)
                && got.contains(&Rule::LowContrast)
                && got.contains(&Rule::SmallTarget),
            "{label} bypassed layout checks: got {got:?}, probe: {:#?}",
            r.probe
        );
    }
}
