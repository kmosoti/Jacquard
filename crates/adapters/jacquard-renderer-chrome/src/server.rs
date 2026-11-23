//! Ephemeral loopback HTTP server for rendering Swatches without `file://`.

use std::collections::BTreeMap;
use std::io::{BufRead as _, BufReader, Write as _};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use jacquard_core::viewport::Viewport;
use jacquard_ports::{Failure, SwatchSource};

use crate::sandbox::{ensure_no_secrets, generate_nonce};
use crate::wrapper::{probe_wrapper, shot_wrapper};

const SWATCH_CSP: &str = "script-src 'none'; object-src 'none'; base-uri 'none'; worker-src 'none'";

struct WrapperPage {
    body: Vec<u8>,
    csp: String,
}

/// Ephemeral `127.0.0.1:0` static HTTP server serving one validated Swatch and
/// unguessable, one-shot, CSP-locked per-viewport wrappers.
pub struct LoopbackServer {
    addr: SocketAddr,
    shot_paths: BTreeMap<String, String>,
    probe_paths: BTreeMap<String, String>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LoopbackServer {
    /// Binds to `127.0.0.1:0` and starts serving `swatch` and one-shot wrappers.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::Unavailable`] if a loopback socket or random nonce fails,
    /// or [`Failure::Failed`] if `swatch` collides with any secret or nonce.
    pub fn start(
        swatch: &SwatchSource,
        viewports: &[Viewport],
        nonce: &str,
        mac_key: &[u8; 32],
    ) -> Result<Self, Failure> {
        let mut wrappers = BTreeMap::new();
        let mut shot_paths = BTreeMap::new();
        let mut probe_paths = BTreeMap::new();
        for &vp in viewports {
            let script_csp = generate_nonce()?;
            let style_csp = generate_nonce()?;
            let shot_tok = generate_nonce()?;
            let probe_tok = generate_nonce()?;
            ensure_no_secrets(swatch, &[&script_csp, &style_csp, &shot_tok, &probe_tok])?;

            let shot_key = format!("__jacquard_shot-{shot_tok}.html");
            let shot_html = shot_wrapper(&swatch.entry, vp, &style_csp);
            let shot_csp = format!(
                "default-src 'none'; style-src 'nonce-{style_csp}'; \
                 frame-src 'self'; base-uri 'none'; object-src 'none'; form-action 'none'"
            );
            wrappers.insert(
                shot_key.clone(),
                WrapperPage {
                    body: shot_html.into_bytes(),
                    csp: shot_csp,
                },
            );
            shot_paths.insert(vp.to_string(), shot_key);

            let probe_key = format!("__jacquard_probe-{probe_tok}.html");
            let (probe_html, probe_csp) =
                probe_wrapper(&swatch.entry, vp, nonce, mac_key, &script_csp, &style_csp);
            wrappers.insert(
                probe_key.clone(),
                WrapperPage {
                    body: probe_html.into_bytes(),
                    csp: probe_csp,
                },
            );
            probe_paths.insert(vp.to_string(), probe_key);
        }

        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| Failure::Unavailable(format!("loopback bind: {e}")))?;
        let addr = listener
            .local_addr()
            .map_err(|e| Failure::Unavailable(format!("loopback addr: {e}")))?;
        let files = Arc::new(swatch.files.clone());
        let wrappers = Arc::new(Mutex::new(wrappers));
        let stop = Arc::new(AtomicBool::new(false));
        let stop_bg = Arc::clone(&stop);

        let thread = std::thread::Builder::new()
            .name("jacquard-chrome-http".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if stop_bg.load(Ordering::Acquire) {
                        break;
                    }
                    if let Ok(stream) = stream {
                        serve_conn(stream, &files, &wrappers);
                    }
                }
            })
            .map_err(|e| Failure::Unavailable(format!("http thread: {e}")))?;

        Ok(Self {
            addr,
            shot_paths,
            probe_paths,
            stop,
            thread: Some(thread),
        })
    }

    /// Returns the unguessable screenshot wrapper URL for `viewport`.
    #[must_use]
    pub fn shot_url(&self, viewport: Viewport) -> String {
        let key = &self.shot_paths[&viewport.to_string()];
        format!("http://{}/{key}", self.addr)
    }

    /// Returns the unguessable probe wrapper URL for `viewport`.
    #[must_use]
    pub fn probe_url(&self, viewport: Viewport) -> String {
        let key = &self.probe_paths[&viewport.to_string()];
        format!("http://{}/{key}", self.addr)
    }
}

impl Drop for LoopbackServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_millis(200));
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

fn serve_conn(
    mut stream: TcpStream,
    files: &BTreeMap<String, Vec<u8>>,
    wrappers: &Mutex<BTreeMap<String, WrapperPage>>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    let mut req_line = String::new();
    if reader.read_line(&mut req_line).is_err() {
        return;
    }
    // Drain headers up to blank line (bounded so oversized headers cannot hang).
    for _ in 0..128 {
        let mut hdr = String::new();
        match reader.read_line(&mut hdr) {
            Ok(0) | Err(_) => break,
            Ok(_) if hdr == "\r\n" || hdr == "\n" => break,
            Ok(_) => {}
        }
    }

    let mut parts = req_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_target = parts.next().unwrap_or("/");
    if method != "GET" && method != "HEAD" {
        write_response(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            None,
            b"",
            true,
        );
        return;
    }
    let path_only = raw_target
        .split(['?', '#'])
        .next()
        .unwrap_or("/")
        .trim_start_matches('/');
    let decoded = percent_decode(path_only);
    let head_only = method == "HEAD";

    // Wrappers are strictly one-shot on GET so iframe script cannot re-fetch CSP/MAC secrets.
    if method == "GET"
        && let Some(page) = wrappers.lock().ok().and_then(|mut m| m.remove(&decoded))
    {
        write_response(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            Some(&page.csp),
            &page.body,
            false,
        );
        return;
    }
    if SwatchSource::validate_path(&decoded).is_ok()
        && let Some(body) = files.get(&decoded)
    {
        let ctype = content_type(&decoded);
        write_response(
            &mut stream,
            "200 OK",
            ctype,
            Some(SWATCH_CSP),
            body,
            head_only,
        );
        return;
    }
    write_response(
        &mut stream,
        "404 Not Found",
        "text/plain",
        None,
        b"not found",
        head_only,
    );
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    csp: Option<&str>,
    body: &[u8],
    head_only: bool,
) {
    let csp_hdr = csp.map_or_else(String::new, |c| format!("Content-Security-Policy: {c}\r\n"));
    let header = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         {csp_hdr}\
         Cache-Control: no-store\r\n\
         X-Content-Type-Options: nosniff\r\n\
         Referrer-Policy: no-referrer\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    if !head_only {
        let _ = stream.write_all(body);
    }
    let _ = stream.flush();
}

fn content_type(path: &str) -> &'static str {
    let ext = path
        .rsplit_once('.')
        .map_or("", |(_, e)| e)
        .to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push((hi << 4) | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
