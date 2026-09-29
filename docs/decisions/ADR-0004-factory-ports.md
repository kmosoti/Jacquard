# ADR-0004: The Factory's Ports and Their First Adapters

- **Status.** Accepted
- **Date.** 2026-09-28
- **Provenance.** [ADR-0000](ADR-0000-foundations.md), the [loop view](../architecture/loop.md), and probes run on the owner's machine on 2026-09-28 (Chrome for Testing 151 headless; the Claude CLI 2.1.283 serving `claude-sonnet-5-5`).

## Context

The cascade needs five effects: weave a Swatch, render it, run a Check on it, ask a Judge about two Renders, and keep the records. Each is a port so that the use cases never know which technology performs it. The first adapters must run on one WSL2 machine with no Anthropic API key, a cached Chrome, and the Claude CLI signed in to the owner's account.

## Decision

### Ports

| Port | Contract |
| --- | --- |
| `Loom` | Weaves one Swatch from a Draft and a seed. Proposes; never verifies; never sees a Judgment. |
| `Renderer` | Renders a Swatch at each audit viewport into a screenshot and a probe report, or reports that it does not render. The probe report is measured by the page itself, in the page, after layout. |
| `Checker` | Runs one registered Check and returns its Outcome. |
| `Judge` | Returns one Judgment for two Renders in the presented order. It receives screenshots and the rubric, nothing else. |
| `Ledger` | Appends Receipts, Judgments, Votes and admission reports; reads them back. Never rewrites. |

Port methods take and return core types or plain data. No port exposes a technology's types.

### Layout Probes Are Checks, Computed in Core

The renderer injects one probe script into the page. The script measures, at each viewport: horizontal overflow of the document, text clipped by its own box, overlapping text boxes, text contrast against the nearest opaque background, and interactive targets smaller than 24 by 24 CSS pixels (WCAG 2.2 target size, minimum). The script only measures; `jacquard-core` decides. The probe checks are pure functions from a probe report to an Outcome and findings, so their rules are tested without a browser.

### First Adapters

- `jacquard-ledger-fs`: NDJSON files, one per record kind, appended.
- `jacquard-renderer-chrome`: the Chrome binary's own `--headless --screenshot` and `--dump-dom`, one process per viewport, under the machine-wide browser lock `/tmp/installation-browser.lock`. No Node. The binary is `JACQUARD_CHROME`, else the cached Chrome for Testing.
- `jacquard-judge-claude`: the Claude CLI in print mode with stream-JSON input, pinned to `claude-sonnet-5-5`, with the rubric as the whole system prompt, no tools, no MCP servers (`--strict-mcp-config` with an empty configuration), project settings only, no session persistence, run from an empty temporary directory so no CLAUDE.md or memory is found. Images travel as base64 content blocks. Each response records the model that actually answered; a Judgment answered by any other model is discarded and reported.
- The `Loom` and `Checker` adapters arrive with the first Product (milestone 05), which is their first real use.

### The Alias Is Not the Model

On 2026-09-28 the CLI alias `sonnet` answered as `claude-sonnet-5`, not Sonnet 5.5. The design workflow therefore pins `claude-sonnet-5-5` by identifier everywhere and never uses an alias.

## Consequences

Easier: the Judge's context is exactly the rubric and two sets of screenshots, verified per call. Harder: each Judgment starts a CLI process (seconds, about four US cents on the probe); the transport depends on the owner's CLI login until an API key exists, and swapping to an API adapter is a new adapter crate, not a change to the port.

## Validation

Falsified if a Judgment is recorded whose answering model is not `claude-sonnet-5-5`, or whose request carried anything beyond the rubric and the renders.
