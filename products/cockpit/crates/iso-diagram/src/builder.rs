//! Fluent strongly typed builder API backing [`crate::model::Diagram::builder`] and `iso_diagram!`.

use std::collections::HashMap;

use crate::model::{
    Asset, Diagram, EdgeId, EdgeSpec, EdgeStyle, NodeId, NodeSpec, NodeStatus, ThemePreset, Tint,
    ZoneId, ZoneSpec, ZoneStyle, sanitize_slug,
};

/// Fluent strongly typed diagram builder.
#[derive(Debug, Clone)]
pub struct DiagramBuilder {
    diagram: Diagram,
    node_slugs: HashMap<String, NodeId>,
}

impl DiagramBuilder {
    /// Creates an empty diagram builder with [`ThemePreset::WarmPaper`] defaults.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            diagram: Diagram {
                title: title.into(),
                theme: ThemePreset::WarmPaper,
                zones: Vec::new(),
                nodes: Vec::new(),
                edges: Vec::new(),
            },
            node_slugs: HashMap::new(),
        }
    }

    /// Sets the diagram theme preset.
    pub fn theme(&mut self, theme: ThemePreset) -> &mut Self {
        self.diagram.theme = theme;
        self
    }

    /// Inserts a named floor zone directly and returns its [`ZoneId`].
    pub fn add_zone(
        &mut self,
        slug: impl Into<String>,
        title: impl Into<String>,
        style: ZoneStyle,
        tint: Tint,
        parent: Option<ZoneId>,
    ) -> ZoneId {
        let id = ZoneId(self.diagram.zones.len());
        self.diagram.zones.push(ZoneSpec {
            id,
            slug: slug.into(),
            title: title.into(),
            tint,
            style,
            parent,
        });
        id
    }

    /// Updates a zone's floor mat stroke style.
    pub fn set_zone_style(&mut self, id: ZoneId, style: ZoneStyle) {
        self.diagram.zones[id.0].style = style;
    }

    /// Updates a zone's semantic tint.
    pub fn set_zone_tint(&mut self, id: ZoneId, tint: Tint) {
        self.diagram.zones[id.0].tint = tint;
    }

    /// Opens a fluent top-level zone closure and returns its [`ZoneId`].
    pub fn zone<F>(&mut self, title: impl Into<String>, f: F) -> ZoneId
    where
        F: FnOnce(&mut ZoneBuilder<'_>),
    {
        let title_str = title.into();
        let slug = sanitize_slug(&title_str);
        let zid = self.add_zone(slug, title_str, ZoneStyle::Solid, Tint::Slate, None);
        let mut zb = ZoneBuilder {
            builder: self,
            zone_id: zid,
        };
        f(&mut zb);
        zid
    }

    /// Inserts a node with an explicit slug and optional zone membership.
    pub fn insert_node(
        &mut self,
        slug: impl Into<String>,
        asset: Asset,
        title: impl Into<String>,
        zone: Option<ZoneId>,
    ) -> NodeRef<'_> {
        let id = NodeId(self.diagram.nodes.len());
        let slug_s = slug.into();
        self.node_slugs.insert(slug_s.clone(), id);
        self.diagram.nodes.push(NodeSpec {
            id,
            slug: slug_s,
            asset,
            title: title.into(),
            subtitle: None,
            zone,
            tint: None,
            status: NodeStatus::Healthy,
            grid_hint: None,
        });
        NodeRef { builder: self, id }
    }

    /// Inserts a top-level node whose slug is derived from `title`.
    pub fn node(&mut self, asset: Asset, title: impl Into<String>) -> NodeRef<'_> {
        let title_s = title.into();
        let slug = sanitize_slug(&title_s);
        self.insert_node(slug, asset, title_s, None)
    }

    /// Adds a directed edge between `from` and `to`.
    pub fn edge(&mut self, from: NodeId, to: NodeId) -> EdgeRef<'_> {
        let id = EdgeId(self.diagram.edges.len());
        let tint = self.diagram.nodes[from.0]
            .tint
            .or_else(|| {
                self.diagram.nodes[from.0]
                    .zone
                    .map(|z| self.diagram.zones[z.0].tint)
            })
            .unwrap_or(Tint::Cobalt);
        self.diagram.edges.push(EdgeSpec {
            id,
            from,
            to,
            style: EdgeStyle::Solid,
            tint,
            label: None,
            bidirectional: false,
        });
        EdgeRef { builder: self, id }
    }

    /// Adds a labeled directed edge between `from` and `to`.
    pub fn connect(&mut self, from: NodeId, to: NodeId, label: impl Into<String>) -> EdgeRef<'_> {
        let label_s = label.into();
        let er = self.edge(from, to);
        if !label_s.is_empty() {
            er.builder.diagram.edges[er.id.0].label = Some(label_s);
        }
        er
    }

    /// Finalizes and consumes the builder into a [`Diagram`].
    pub fn build(self) -> Diagram {
        self.diagram
    }

    #[doc(hidden)]
    pub fn node_mut_internal(&mut self, id: NodeId) -> &mut NodeSpec {
        &mut self.diagram.nodes[id.0]
    }

    #[doc(hidden)]
    pub fn edge_mut_internal(&mut self, id: EdgeId) -> &mut EdgeSpec {
        &mut self.diagram.edges[id.0]
    }
}

/// Zone-scoped builder that automatically assigns child nodes and sub-zones.
pub struct ZoneBuilder<'a> {
    /// Underlying diagram builder.
    pub builder: &'a mut DiagramBuilder,
    /// Identifier of the zone currently being populated.
    pub zone_id: ZoneId,
}

impl<'a> ZoneBuilder<'a> {
    /// Sets the enclosing zone border style.
    pub fn style(&mut self, style: ZoneStyle) -> &mut Self {
        self.builder.set_zone_style(self.zone_id, style);
        self
    }

    /// Sets the enclosing zone tint.
    pub fn tint(&mut self, tint: Tint) -> &mut Self {
        self.builder.set_zone_tint(self.zone_id, tint);
        self
    }

    /// Opens a nested child subzone inside the current zone.
    pub fn subzone<F>(&mut self, title: impl Into<String>, f: F) -> ZoneId
    where
        F: FnOnce(&mut ZoneBuilder<'_>),
    {
        let title_s = title.into();
        let slug = sanitize_slug(&title_s);
        let zid = self.builder.add_zone(
            slug,
            title_s,
            ZoneStyle::Solid,
            self.builder.diagram.zones[self.zone_id.0].tint,
            Some(self.zone_id),
        );
        let mut child = ZoneBuilder {
            builder: self.builder,
            zone_id: zid,
        };
        f(&mut child);
        zid
    }

    /// Adds a node inside this zone with a slug derived from `title`.
    pub fn add(&mut self, asset: Asset, title: impl Into<String>) -> NodeRef<'_> {
        let title_s = title.into();
        let slug = sanitize_slug(&title_s);
        self.builder
            .insert_node(slug, asset, title_s, Some(self.zone_id))
    }

    /// Adds a node inside this zone with an explicit slug.
    pub fn add_with_slug(
        &mut self,
        slug: impl Into<String>,
        asset: Asset,
        title: impl Into<String>,
    ) -> NodeRef<'_> {
        self.builder
            .insert_node(slug, asset, title, Some(self.zone_id))
    }

    /// Returns the identifier of this zone.
    pub fn id(&self) -> ZoneId {
        self.zone_id
    }
}

/// Borrowed fluent customizer returned when adding a node.
pub struct NodeRef<'a> {
    builder: &'a mut DiagramBuilder,
    id: NodeId,
}

impl<'a> NodeRef<'a> {
    /// Overrides the node's generated slug.
    pub fn slug(self, slug: impl Into<String>) -> Self {
        self.builder.diagram.nodes[self.id.0].slug = slug.into();
        self
    }

    /// Attaches a secondary telemetry subtitle beneath the node title.
    pub fn subtitle(self, subtitle: impl Into<String>) -> Self {
        let s = subtitle.into();
        if !s.is_empty() {
            self.builder.diagram.nodes[self.id.0].subtitle = Some(s);
        }
        self
    }

    /// Sets an explicit node accent tint overriding its zone tint.
    pub fn tint(self, tint: Tint) -> Self {
        self.builder.diagram.nodes[self.id.0].tint = Some(tint);
        self
    }

    /// Sets the node's operational health status.
    pub fn status(self, status: NodeStatus) -> Self {
        self.builder.diagram.nodes[self.id.0].status = status;
        self
    }

    /// Sets an optional explicit `(col, row)` placement hint for Sugiyama tiering.
    pub fn at_grid(self, col: usize, row: usize) -> Self {
        self.builder.diagram.nodes[self.id.0].grid_hint = Some((col, row));
        self
    }

    /// Finishes customizing and returns the [`NodeId`].
    pub fn id(self) -> NodeId {
        self.id
    }
}

/// Borrowed fluent customizer returned when creating an edge.
pub struct EdgeRef<'a> {
    builder: &'a mut DiagramBuilder,
    id: EdgeId,
}

impl<'a> EdgeRef<'a> {
    /// Sets the edge stroke pattern.
    pub fn style(self, style: EdgeStyle) -> Self {
        self.builder.diagram.edges[self.id.0].style = style;
        self
    }

    /// Sets the edge tint.
    pub fn tint(self, tint: Tint) -> Self {
        self.builder.diagram.edges[self.id.0].tint = tint;
        self
    }

    /// Attaches an edge badge label.
    pub fn label(self, label: impl Into<String>) -> Self {
        let s = label.into();
        if !s.is_empty() {
            self.builder.diagram.edges[self.id.0].label = Some(s);
        }
        self
    }

    /// Marks the link as bidirectional.
    pub fn bidirectional(self, bidirectional: bool) -> Self {
        self.builder.diagram.edges[self.id.0].bidirectional = bidirectional;
        self
    }

    /// Finishes customizing and returns the [`EdgeId`].
    pub fn id(self) -> EdgeId {
        self.id
    }
}
