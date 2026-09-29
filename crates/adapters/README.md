# Adapters

One crate per port and technology, named `jacquard-<port>-<technology>`. An adapter depends on `jacquard-core` and the one port it implements, and on nothing else in the workspace. Only `crates/bin/` wires adapters to ports.

Planned:

| Crate | Port | Technology |
| --- | --- | --- |
| `jacquard-ledger-fs` | Ledger | Append-only NDJSON files under `verification/` |
| `jacquard-loom-anthropic` | Loom | Claude Sonnet 5.5 as generator |
| `jacquard-judge-anthropic` | Judge | Claude Sonnet 5.5 as the sole pairwise judge (ADR-0002) |
| `jacquard-renderer-playwright` | Renderer | Headless Chromium at the audit viewports |
| `jacquard-checker-process` | Checker | Registered commands: axe, Lighthouse CI, layout probes, token lint |
| `jacquard-judge-local` | Judge (structured filter only) | llama.cpp with a small VLM; never a ranking judge |
