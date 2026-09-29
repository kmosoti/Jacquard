//! A Draft is compiled design intent: the product contract's constraints,
//! the design tokens, the content, and the audit viewports for one surface,
//! authored in typed Rust and consumed by a Loom as an inert artifact.
//!
//! The name is the weaving draft, the notation that tells a loom which
//! threads lift. A Draft says what an interface must be; it never says how
//! to generate one.

/// Identifies a Draft by the content hash of its canonical encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DraftId(pub [u8; 32]);
