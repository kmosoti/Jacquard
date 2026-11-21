#![forbid(unsafe_code)]
//! Zero-dependency declarative 2.5D isometric infrastructure diagramming library.
//!
//! Provides a strongly typed [`iso_diagram!`] eDSL, a deterministic layout &
//! orthogonal conduit router ([`layout`]), 360-degree isometric orbit camera math
//! ([`projection`] & [`camera`]), 12 procedural 2.5D hardware sprite primitives
//! ([`assets`]), and a standalone Warm Paper / Obsidian Slate SVG renderer ([`svg`]).

pub mod assets;
pub mod builder;
pub mod camera;
pub mod catalog;
pub mod dsl;
pub mod geometry;
pub mod layout;
pub mod layout_tiers;
pub mod model;
pub mod projection;
pub mod routing;
pub mod svg;
pub mod svg_palette;

/// Convenience imports for building diagrams with [`iso_diagram!`] or [`Diagram::builder`].
pub mod prelude {
    pub use crate::dsl::emit_rust_dsl;
    pub use crate::iso_diagram;
    pub use crate::model::{
        Asset, Diagram, EdgeId, EdgeStyle, NodeId, NodeStatus, ThemePreset, Tint, ZoneId,
        ZoneStyle, drum, rack, rack_pedestal, switch,
    };
}
