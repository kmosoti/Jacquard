# Hexagon

Domain semantics point inward; effects point outward ([ADR-0000](../decisions/ADR-0000-foundations.md)).

```mermaid
flowchart LR
    cli["jacquard<br/>bin"] --> app["jacquard-app<br/>app"]
    cli --> ledger["jacquard-ledger-fs<br/>adapter"]
    cli --> chrome["jacquard-renderer-chrome<br/>adapter"]
    cli --> claude["jacquard-judge-claude<br/>adapter"]
    xtask["jacquard-xtask<br/>bin"]
    app --> core["jacquard-core<br/>core"]
    app --> ports["jacquard-ports<br/>ports"]
    ledger -.-> ports
    chrome -.-> ports
    claude -.-> ports
    ports --> core
```

| Layer | Crates | May depend on | Holds |
| --- | --- | --- | --- |
| core | `jacquard-core` | nothing | Draft, Swatch, Outcome, Judgment, Vote, viewports, probe rules, the admission kernel; `no_std`, pure |
| ports | `jacquard-ports` | core | Loom, Renderer, Checker, Judge, Ledger traits |
| app | `jacquard-app` | core, ports | the cascade as use cases |
| adapters | `jacquard-ledger-fs`, `jacquard-renderer-chrome`, `jacquard-judge-claude`; later a Loom, a process Checker and a local structured filter | core and one port | technology |
| bin | `jacquard`, `jacquard-xtask` | anything | composition root; repository tooling |

Dev, build and target-specific dependencies get no exemption. Nothing depends on a `bin/` crate. A new crate needs a layer in `layers.json`; an exception needs a written reason in an ADR. Workspace members are globs by layer directory, so adding a crate never edits the root manifest.
