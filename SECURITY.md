# Security Policy

Jacquard orchestrates headless browser rendering, Markdown/HTML rendering, external coding-agent workspaces, and build-time UI verification pipelines. Security reports are prioritized and handled before public disclosure.

## Supported Versions

Jacquard is in pre-release alpha (`0.1.1-alpha.1` line). Only `main` and the latest tagged alpha release receive security fixes.

| Version | Supported |
| --- | --- |
| `main` | Yes |
| Latest `v0.1.1-alpha.*` release | Yes |
| Earlier pre-release tags | No |

## Trust Boundaries & Runtime Isolation (`0.1.1-alpha.1`)

- **Swatch & CLI Ingestion (`jacquard-ports`, `jacquard` CLI):** `SwatchSource::validate` rejects empty, rooted/absolute, backslash, `:`/drive-prefix, control/NUL, `.`/`..` paths and requires `entry` inside `files`. The CLI `swatch_loader` walks directories via `symlink_metadata` + canonical root containment + file identity checks, rejecting all symlinks and non-regular files and bounding file counts (`<= 512`) and total bytes (`<= 32 MiB`).
- **Headless Chrome Renderer (`jacquard-renderer-chrome`):**
  - Candidate Swatches are materialized into `0700` scratch directories via no-follow containment checks and served over an ephemeral `127.0.0.1:0` loopback HTTP listener (never `file://`).
  - Chrome launches with an isolated `0700` `--user-data-dir`, `--disable-background-networking`, `--disable-extensions`, `--proxy-server=direct://`, and `--host-resolver-rules=MAP * ~NOTFOUND , EXCLUDE 127.0.0.1 , EXCLUDE localhost`.
  - The Chromium sandbox is **enabled by default**. Setting `JACQUARD_ALLOW_NO_SANDBOX=1` opts in to `--no-sandbox` strictly for restricted CI containers (such as Ubuntu 24.04 AppArmor unprivileged-userns restrictions after probing native sandbox launch).
  - Candidate Swatches are framed inside `<iframe sandbox="allow-same-origin">` (without `allow-scripts`) and served with `Content-Security-Policy: script-src 'none'; object-src 'none'; base-uri 'none'; worker-src 'none'`, preventing candidate JavaScript execution and candidate-realm DOM/prototype spoofing while preserving same-origin layout measurement.
  - Layout probe measurements run inside an outer CSP-locked (`default-src 'none'; script-src 'nonce-...'`), strictly one-shot wrapper using frozen host DOM/JS intrinsics after synchronous wrapper `<script>` removal (`currentScript.remove()`), tear down the candidate iframe and sanitize parent DOM nodes before emission, and authenticate `<script type="application/json" id="jacquard-probe-{nonce}" data-sig="{hmac}">` with pure-JS zero-prototype HMAC-SHA256 verified in constant time in Rust. Candidate tampering fails closed.
- **Playwright Renderer (`jacquard-renderer-playwright`):** Blocks non-target origins via `context.route("**/*", ...)` plus resolver/proxy isolation flags, and evaluates `probe.js` inside a fresh CDP `Page.createIsolatedWorld` (`Runtime.evaluate` with `returnByValue: true`) so main-world JavaScript/WASM cannot tamper with probe prototypes or `#jacquard-probe` DOM nodes.
- **Loom Workspace Preparation (`jacquard-loom-claude`):** Rejects tracked and untracked symlinks and non-regular files via `symlink_metadata` and canonical containment checks, and excludes sensitive untracked credential/secret files (`.env`, `.env.*`, `*.pem`, `*.key`, `*secret*`, `*credential*`, `*token*`, `.npmrc`, `.netrc`, `id_rsa*`, `id_ed25519*`) from entering the external model workspace.
- **Cockpit Wiki Engine (`cockpit-wiki`):** Flattens all inline/link events inside Markdown image descriptions to HTML-escaped plain text, rejects whitespace/quotes/angle-brackets/backslashes/control characters in URLs, and enforces Mermaid layout budgets (`<= 16 KiB`, `<= 64` nodes, `<= 128` edges, `<= 128` label chars, `<= 32` zones) before invoking `iso-diagram`.
- **Verification Gates & Release Provenance (`jacquard-xtask`, `.github/workflows/release.yml`):** `check-core-purity` tokenizes Rust sources (ignoring spoofed attributes in comments/strings, checking `[lints] workspace = true`, rejecting `#[path]`/`include*!` escapes outside crate `src/`, and detecting `use std` / `::std::` / `extern crate std`); `check-trust-boundary` and `mutants semantic` reject empty corpora and unauthorized mutant deletions/weakening; release tags require `origin/main` ancestry, tag-to-manifest version matching, and separate unprivileged `build` (`contents: read`) from minimal `publish` (`contents: write`, `id-token: write`, `attestations: write`).

## Reporting a Vulnerability

**Do not report vulnerabilities in public issues, discussions, or pull requests.**

Report vulnerabilities privately through GitHub's [private vulnerability reporting](https://github.com/kmosoti/Jacquard/security/advisories/new) for this repository.

Please include:

- affected crate or product (`jacquard-core`, `jacquard-renderer-chrome`, `cockpit-wiki`, `cockpit-leptos`, etc.)
- security impact (path traversal, script/HTML injection, command injection, trust-boundary bypass, or artifact tampering)
- minimal reproduction steps or proof-of-concept input

## What to Expect

- Initial acknowledgement within **5 business days**
- Assessment and remediation plan within **30 days**
- Coordinated disclosure once a fix and regression test are landed

## Scope

In scope:

- `cockpit-wiki` path-traversal sanitization, Markdown/HTML/URL XSS sanitization, and Mermaid DoS budgets
- `jacquard-xtask` trust-boundary, core-purity, semantic-mutant inventory, and receipt validation
- `jacquard` CLI / renderer sandbox boundaries, unforgeable probe integrity, Loom workspace containment, and release provenance artifacts (`SHA256SUMS`, SLSA attestations)

Out of scope:

- Vulnerabilities in Chromium/Chrome, Playwright, or third-party Rust crates (report upstream)
- Local developer machines where an attacker already has local shell write access
