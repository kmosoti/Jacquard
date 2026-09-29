# Adapters

One crate per port and technology, named `jacquard-<port>-<technology>`. An adapter depends on `jacquard-core` and the one port it implements, and on nothing else in the workspace. Only `crates/bin/` wires adapters to ports.

| Crate | Port | Technology |
| --- | --- | --- |
| `jacquard-ledger-fs` | Ledger | Append-only NDJSON files |
| `jacquard-renderer-chrome` | Renderer | Chrome's headless `--screenshot` and `--dump-dom`, the page framed at the exact viewport, with an in-page layout probe |
| `jacquard-judge-claude` | Judge | Claude Sonnet 5.5 (`claude-sonnet-5-5`, pinned) through the Claude CLI with no tools, no MCP servers and an empty working directory |

Planned: a Loom and a process Checker with the first Product (milestone 05), and a small local model as a structured filter (milestone 06), never a ranking Judge.
