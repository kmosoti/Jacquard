//! Strongly typed core data model for declarative isometric diagrams.

/// Compile-time / builder handle identifying a node in a [`Diagram`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub usize);

/// Handle identifying a logical zone (floor mat) in a [`Diagram`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZoneId(pub usize);

/// Handle identifying a routed connection in a [`Diagram`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(pub usize);

/// Procedural 2.5D hardware sprite primitives bundled in the crate asset library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Asset {
    /// Multi-blade server rack, optionally mounted on a circular pedestal.
    Rack {
        /// Number of chassis blades rendered (clamped 1..=5).
        blades: u8,
        /// Whether to render a pedestal ring beneath the rack.
        pedestal: bool,
    },
    /// Ribbed cylindrical storage unit.
    Drum {
        /// Number of horizontal ribbed bands (clamped 2..=4).
        rings: u8,
    },
    /// Cartridge bin / backup crate.
    Crate,
    /// Segmented masonry policy firewall.
    Firewall,
    /// Rack-mount multi-port network switch.
    Switch {
        /// Number of front RJ45/SFP ports rendered (clamped 4..=12).
        ports: u8,
    },
    /// Dual-antenna gateway router.
    Router,
    /// Workstation laptop client.
    Laptop,
    /// Padlock / cryptographic boundary element.
    Lock,
    /// Silicon die / enclave processor package.
    Chip,
    /// WAN or external network cloud cluster.
    Cloud,
    /// Compact edge pod / single-board node.
    Pod,
    /// Cantilevered traffic load balancer.
    Balancer,
}

impl Asset {
    /// Complete catalog of all 12 hardware sprites with standard defaults.
    pub const ALL: &'static [Asset] = &[
        Asset::Rack {
            blades: 3,
            pedestal: true,
        },
        Asset::Drum { rings: 3 },
        Asset::Crate,
        Asset::Firewall,
        Asset::Switch { ports: 8 },
        Asset::Router,
        Asset::Laptop,
        Asset::Lock,
        Asset::Chip,
        Asset::Cloud,
        Asset::Pod,
        Asset::Balancer,
    ];

    /// Short canonical identifier used by DSL exporters and galleries.
    pub fn kind_name(self) -> &'static str {
        match self {
            Asset::Rack { .. } => "Rack",
            Asset::Drum { .. } => "Drum",
            Asset::Crate => "Crate",
            Asset::Firewall => "Firewall",
            Asset::Switch { .. } => "Switch",
            Asset::Router => "Router",
            Asset::Laptop => "Laptop",
            Asset::Lock => "Lock",
            Asset::Chip => "Chip",
            Asset::Cloud => "Cloud",
            Asset::Pod => "Pod",
            Asset::Balancer => "Balancer",
        }
    }

    /// Formats the typed Rust expression reproducing this asset variant.
    pub fn rust_expr(self) -> String {
        match self {
            Asset::Rack { blades, pedestal } => {
                format!("Rack {{ blades: {blades}, pedestal: {pedestal} }}")
            }
            Asset::Drum { rings } => format!("Drum {{ rings: {rings} }}"),
            Asset::Switch { ports } => format!("Switch {{ ports: {ports} }}"),
            other => other.kind_name().to_string(),
        }
    }
}

/// Convenience constructor for a non-pedestal multi-blade server rack.
pub fn rack(blades: u8) -> Asset {
    Asset::Rack {
        blades,
        pedestal: false,
    }
}

/// Convenience constructor for a pedestal-mounted multi-blade server rack.
pub fn rack_pedestal(blades: u8) -> Asset {
    Asset::Rack {
        blades,
        pedestal: true,
    }
}

/// Convenience constructor for a ribbed cylindrical storage drum.
pub fn drum(rings: u8) -> Asset {
    Asset::Drum { rings }
}

/// Convenience constructor for a rack-mount network switch.
pub fn switch(ports: u8) -> Asset {
    Asset::Switch { ports }
}

/// Floor mat border and fill presentation for a [`ZoneSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ZoneStyle {
    /// Filled floor mat with solid boundary stroke.
    #[default]
    Solid,
    /// Lightly tinted floor mat with dashed border.
    Dashed,
    /// Transparent wireframe boundary outline.
    Wireframe,
}

/// Orthogonal pipe stroke pattern for an [`EdgeSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EdgeStyle {
    /// Continuous link conduit.
    #[default]
    Solid,
    /// Dashed tunnel or control link.
    Dashed,
    /// Dotted virtual / overlay link.
    Dotted,
}

/// Semantic color family mapped onto the active theme palette at render time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tint {
    /// Neutral warm stone / cool slate.
    #[default]
    Slate,
    /// Forest pine green.
    Pine,
    /// Warm industrial amber.
    Amber,
    /// Alert crimson red.
    Crimson,
    /// Deep steel cobalt blue.
    Cobalt,
    /// Muted plum violet.
    Plum,
    /// Pale sage green.
    Sage,
}

/// Operational health state of a node (drives highlight rings and indicators).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NodeStatus {
    /// Nominal operating state.
    #[default]
    Healthy,
    /// Actively serving or highlighted.
    Active,
    /// Experiencing recoverable backpressure or lag.
    Degraded,
    /// Offline or faulted.
    Failed,
    /// Awaiting initial telemetry sync.
    Pending,
}

/// Color preset used by standalone SVG rendering and default diagram styling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemePreset {
    /// Warm drafting paper daylight theme.
    #[default]
    WarmPaper,
    /// Low-glare obsidian slate dark theme.
    ObsidianSlate,
}

/// Rectangular ground zone containing related infrastructure assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneSpec {
    /// Stable identifier within the parent [`Diagram`].
    pub id: ZoneId,
    /// Normalized Rust/CSS-friendly identifier slug.
    pub slug: String,
    /// Human-readable display label.
    pub title: String,
    /// Semantic tint applied to fill and border.
    pub tint: Tint,
    /// Stroke presentation style.
    pub style: ZoneStyle,
    /// Enclosing parent zone when nested.
    pub parent: Option<ZoneId>,
}

/// Individual asset node placed on the isometric campus floor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSpec {
    /// Stable identifier within the parent [`Diagram`].
    pub id: NodeId,
    /// Normalized Rust/CSS-friendly identifier slug.
    pub slug: String,
    /// Procedural 2.5D hardware sprite variant.
    pub asset: Asset,
    /// Primary node title.
    pub title: String,
    /// Optional compact telemetry subtitle.
    pub subtitle: Option<String>,
    /// Containing zone if placed inside a bay/zone.
    pub zone: Option<ZoneId>,
    /// Optional explicit tint overriding zone default.
    pub tint: Option<Tint>,
    /// Operational health state.
    pub status: NodeStatus,
    /// Optional explicit `(col, row)` campus layout hint.
    pub grid_hint: Option<(usize, usize)>,
}

/// Orthogonal pipe connection between two nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeSpec {
    /// Stable identifier within the parent [`Diagram`].
    pub id: EdgeId,
    /// Source node identifier.
    pub from: NodeId,
    /// Destination node identifier.
    pub to: NodeId,
    /// Stroke pattern for the conduit.
    pub style: EdgeStyle,
    /// Semantic conduit tint.
    pub tint: Tint,
    /// Optional badge label along the conduit.
    pub label: Option<String>,
    /// Whether flow arrows apply symmetrically.
    pub bidirectional: bool,
}

/// Complete declarative isometric infrastructure diagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagram {
    /// Scene title.
    pub title: String,
    /// Default color preset.
    pub theme: ThemePreset,
    /// Ordered floor zones.
    pub zones: Vec<ZoneSpec>,
    /// Ordered hardware nodes.
    pub nodes: Vec<NodeSpec>,
    /// Ordered orthogonal links.
    pub edges: Vec<EdgeSpec>,
}

impl Diagram {
    /// Starts a fluent [`DiagramBuilder`] for constructing a scene programmatically.
    pub fn builder(title: impl Into<String>) -> crate::builder::DiagramBuilder {
        crate::builder::DiagramBuilder::new(title)
    }

    /// Borrows a [`NodeSpec`] by handle.
    pub fn node(&self, id: NodeId) -> &NodeSpec {
        &self.nodes[id.0]
    }

    /// Borrows a [`ZoneSpec`] by handle.
    pub fn zone(&self, id: ZoneId) -> &ZoneSpec {
        &self.zones[id.0]
    }

    /// Finds the first node matching `slug`, if present.
    pub fn find_node_by_slug(&self, slug: &str) -> Option<NodeId> {
        self.nodes.iter().find(|n| n.slug == slug).map(|n| n.id)
    }
}

/// Normalizes an arbitrary label into a lowercase Rust-friendly slug.
pub fn sanitize_slug(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_underscore = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_underscore = false;
        } else if !prev_underscore && !out.is_empty() {
            out.push('_');
            prev_underscore = true;
        }
    }
    if out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "item".to_string()
    } else if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("n_{out}")
    } else {
        out
    }
}
