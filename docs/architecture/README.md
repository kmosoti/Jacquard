# Architecture

Views of what exists. Each diagram renders on GitHub; each view names the crates it describes.

- [Hexagon](hexagon.md): the layers, the dependency rule, and which crate sits where. [layers.json](layers.json) is the machine-readable assignment that `cargo xtask check-layers` (milestone 02) enforces.
- [Loop](loop.md): the cascade from Draft to accepted Swatch, and which tier gates.
