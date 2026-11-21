# Security Policy

Jacquard orchestrates headless browser rendering, Markdown/HTML rendering, and build-time UI verification pipelines. Security reports are prioritized and handled before public disclosure.

## Supported Versions

Jacquard is in pre-release alpha (`0.1.0-alpha.1` line). Only `main` and the latest tagged alpha release receive security fixes.

| Version | Supported |
| --- | --- |
| `main` | Yes |
| Latest `v0.1.0-alpha.*` release | Yes |
| Earlier pre-release tags | No |

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

- `cockpit-wiki` path-traversal sanitization and Markdown/HTML/URL XSS sanitization
- `jacquard-xtask` trust-boundary enforcement and receipt validation
- `jacquard` CLI / renderer sandbox boundaries and release provenance artifacts (`SHA256SUMS`, SLSA attestations)

Out of scope:

- Vulnerabilities in Chromium/Chrome, Playwright, or third-party Rust crates (report upstream)
- Local developer machines where an attacker already has local shell write access
