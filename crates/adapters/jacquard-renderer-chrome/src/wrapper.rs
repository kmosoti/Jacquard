//! Outer probe wrapper HTML/JS with CSP, frozen host intrinsics, and HMAC auth.

use jacquard_core::viewport::Viewport;

use crate::PROBE;

/// Prefix for the probe script tag's `id` attribute (`jacquard-probe-<nonce>`).
pub const PROBE_ID_PREFIX: &str = "jacquard-probe-";

const WRAPPER_JS: &str = include_str!("wrapper.js");

/// Builds the static CSP-locked wrapper page used by `--screenshot` renders
/// (contains no probe logic, nonce, or HMAC key).
#[must_use]
pub fn shot_wrapper(entry: &str, viewport: Viewport, style_csp_nonce: &str) -> String {
    let (w, h) = (viewport.width, viewport.height);
    let src = html_attr_escape(&format!("/{entry}"));
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">\
         <style nonce=\"{style_csp_nonce}\">html,body{{margin:0;padding:0;overflow:hidden;background:#fff}}\
         iframe{{display:block;border:0;width:{w}px;height:{h}px}}</style></head>\
         <body><iframe sandbox=\"allow-same-origin\" src=\"{src}\"></iframe></body></html>"
    )
}

/// Builds the outer probe wrapper HTML and matching strict Content Security Policy.
#[must_use]
pub fn probe_wrapper(
    entry: &str,
    viewport: Viewport,
    nonce: &str,
    mac_key: &[u8; 32],
    script_csp_nonce: &str,
    style_csp_nonce: &str,
) -> (String, String) {
    let (w, h) = (viewport.width, viewport.height);
    let entry_url = format!("/{entry}");
    let entry_json = serde_json::to_string(&entry_url).unwrap_or_else(|_| "\"/index.html\"".into());
    let probe_id_json = serde_json::to_string(&format!("{PROBE_ID_PREFIX}{nonce}"))
        .unwrap_or_else(|_| "\"jacquard-probe-\"".into());
    let mac_key_json = serde_json::to_string(&mac_key[..]).unwrap_or_else(|_| "[]".into());

    let listen = WRAPPER_JS
        .replace("__JACQUARD_PROBE_ID_JSON__", &probe_id_json)
        .replace("__JACQUARD_ENTRY_URL_JSON__", &entry_json)
        .replace("__JACQUARD_MAC_KEY_JSON__", &mac_key_json)
        .replace("__JACQUARD_MEASURE_FN__", PROBE);

    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\">\
         <script nonce=\"{script_csp_nonce}\">{listen}</script>\
         <style nonce=\"{style_csp_nonce}\">html,body{{margin:0;padding:0;overflow:hidden;background:#fff}}\
         iframe{{display:block;border:0;width:{w}px;height:{h}px}}</style></head>\
         <body></body></html>"
    );
    let csp = format!(
        "default-src 'none'; script-src 'nonce-{script_csp_nonce}'; \
         style-src 'nonce-{style_csp_nonce}'; frame-src 'self'; \
         base-uri 'none'; object-src 'none'; form-action 'none'"
    );
    (html, csp)
}

fn html_attr_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}
