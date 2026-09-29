//! A Swatch is one candidate interface woven from a Draft: source files and
//! the Draft they answer. Its identity is the hash of its source, so the
//! same source is the same Swatch whichever Loom produced it and however
//! many times.
//!
//! A Swatch is rendered into Renders (screenshots and diagnostics at the
//! audit viewports) by the renderer port. A Swatch that fails to render is
//! rejected before any Check or Judge sees it.

/// Identifies a Swatch by the content hash of its source tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SwatchId(pub [u8; 32]);
