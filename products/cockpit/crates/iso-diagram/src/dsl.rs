//! Strongly typed declarative Rust macro (`iso_diagram!`) and canonical Rust serializer.

use std::collections::HashMap;
use std::fmt::Write;

use crate::model::{Diagram, EdgeStyle, NodeStatus, Tint, ZoneId};

/// Applies identifier attributes onto a node created by `iso_diagram!`.
#[doc(hidden)]
pub fn apply_node_attr(node: &mut crate::model::NodeSpec, attr: &str) {
    match attr {
        "Slate" => node.tint = Some(Tint::Slate),
        "Pine" => node.tint = Some(Tint::Pine),
        "Amber" => node.tint = Some(Tint::Amber),
        "Crimson" => node.tint = Some(Tint::Crimson),
        "Cobalt" => node.tint = Some(Tint::Cobalt),
        "Plum" => node.tint = Some(Tint::Plum),
        "Sage" => node.tint = Some(Tint::Sage),
        "Healthy" => node.status = NodeStatus::Healthy,
        "Active" => node.status = NodeStatus::Active,
        "Degraded" => node.status = NodeStatus::Degraded,
        "Failed" => node.status = NodeStatus::Failed,
        "Pending" => node.status = NodeStatus::Pending,
        other => panic!("unknown iso_diagram! node attribute `{other}`"),
    }
}

/// Applies identifier attributes onto an edge created by `iso_diagram!`.
#[doc(hidden)]
pub fn apply_edge_attr(edge: &mut crate::model::EdgeSpec, attr: &str) {
    match attr {
        "Slate" => edge.tint = Tint::Slate,
        "Pine" => edge.tint = Tint::Pine,
        "Amber" => edge.tint = Tint::Amber,
        "Crimson" => edge.tint = Tint::Crimson,
        "Cobalt" => edge.tint = Tint::Cobalt,
        "Plum" => edge.tint = Tint::Plum,
        "Sage" => edge.tint = Tint::Sage,
        "Solid" => edge.style = EdgeStyle::Solid,
        "Dashed" => edge.style = EdgeStyle::Dashed,
        "Dotted" => edge.style = EdgeStyle::Dotted,
        "Bidi" => edge.bidirectional = true,
        other => panic!("unknown iso_diagram! edge attribute `{other}`"),
    }
}

/// Declarative, compile-time checked Rust diagramming eDSL.
#[macro_export]
macro_rules! iso_diagram {
    (
        title: $title:expr,
        theme: $theme:ident,
        $($body:tt)*
    ) => {{
        use $crate::model::Asset::*;
        use $crate::model::{drum, rack, rack_pedestal, switch};
        let _ = ($crate::model::Asset::Crate, drum, rack, rack_pedestal, switch);
        let mut builder = $crate::builder::DiagramBuilder::new($title);
        builder.theme($crate::model::ThemePreset::$theme);
        $crate::__iso_items!(builder, None; $($body)*);
        builder.build()
    }};
    (
        title: $title:expr,
        $($body:tt)*
    ) => {
        $crate::iso_diagram! {
            title: $title,
            theme: WarmPaper,
            $($body)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __iso_items {
    ($b:ident, $parent:expr;) => {};
    (
        $b:ident, $parent:expr;
        node $var:ident = $asset:expr => ($label:expr $(, $sub:expr)?) $([$($attr:ident),* $(,)?])? ; $($rest:tt)*
    ) => {
        let $var = {
            let mut nr = $b.insert_node(stringify!($var), $asset, $label, $parent);
            $(nr = nr.subtitle($sub);)?
            let id = nr.id();
            $($($crate::dsl::apply_node_attr(
                $b.node_mut_internal(id),
                stringify!($attr),
            );)*)?
            id
        };
        let _ = &$var;
        $crate::__iso_items!($b, $parent; $($rest)*);
    };
    (
        $b:ident, $parent:expr;
        zone $zvar:ident ($ztitle:expr, $zstyle:ident, $ztint:ident) { $($inner:tt)* } $($rest:tt)*
    ) => {
        let $zvar = $b.add_zone(
            stringify!($zvar),
            $ztitle,
            $crate::model::ZoneStyle::$zstyle,
            $crate::model::Tint::$ztint,
            $parent,
        );
        let _ = &$zvar;
        $crate::__iso_items!($b, Some($zvar); $($inner)*);
        $crate::__iso_items!($b, $parent; $($rest)*);
    };
    ($b:ident, $parent:expr; edges { $($edges:tt)* } $($rest:tt)*) => {
        $crate::__iso_edges!($b; $($edges)*);
        $crate::__iso_items!($b, $parent; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __iso_edges {
    ($b:ident;) => {};
    ($b:ident; $from:ident -> $to:ident $([$($attr:ident),* $(,)?])? $(: $label:expr)? ; $($rest:tt)*) => {
        $crate::__iso_push_edge!($b, $from, $to, Solid, false; $([$($attr),*])? $(: $label)?);
        $crate::__iso_edges!($b; $($rest)*);
    };
    ($b:ident; $from:ident -> ($($to:ident),+ $(,)?) $([$($attr:ident),* $(,)?])? $(: $label:expr)? ; $($rest:tt)*) => {
        for to_id in [$($to),+] {
            $crate::__iso_push_edge!($b, $from, to_id, Solid, false; $([$($attr),*])? $(: $label)?);
        }
        $crate::__iso_edges!($b; $($rest)*);
    };
    ($b:ident; $from:ident ~> $to:ident $([$($attr:ident),* $(,)?])? $(: $label:expr)? ; $($rest:tt)*) => {
        $crate::__iso_push_edge!($b, $from, $to, Dashed, false; $([$($attr),*])? $(: $label)?);
        $crate::__iso_edges!($b; $($rest)*);
    };
    ($b:ident; $from:ident ~> ($($to:ident),+ $(,)?) $([$($attr:ident),* $(,)?])? $(: $label:expr)? ; $($rest:tt)*) => {
        for to_id in [$($to),+] {
            $crate::__iso_push_edge!($b, $from, to_id, Dashed, false; $([$($attr),*])? $(: $label)?);
        }
        $crate::__iso_edges!($b; $($rest)*);
    };
    ($b:ident; $from:ident ..> $to:ident $([$($attr:ident),* $(,)?])? $(: $label:expr)? ; $($rest:tt)*) => {
        $crate::__iso_push_edge!($b, $from, $to, Dotted, false; $([$($attr),*])? $(: $label)?);
        $crate::__iso_edges!($b; $($rest)*);
    };
    ($b:ident; $from:ident <-> $to:ident $([$($attr:ident),* $(,)?])? $(: $label:expr)? ; $($rest:tt)*) => {
        $crate::__iso_push_edge!($b, $from, $to, Solid, true; $([$($attr),*])? $(: $label)?);
        $crate::__iso_edges!($b; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __iso_push_edge {
    ($b:ident, $from:ident, $to:ident, $style:ident, $bidi:expr; $([$($attr:ident),*])? $(: $label:expr)?) => {{
        let eid = $b
            .edge($from, $to)
            .style($crate::model::EdgeStyle::$style)
            .bidirectional($bidi)
            $(.label($label))?
            .id();
        $($($crate::dsl::apply_edge_attr(
            $b.edge_mut_internal(eid),
            stringify!($attr),
        );)*)?
    }};
}

/// Serializes any [`Diagram`] into clean, human-readable `iso_diagram! { ... }` Rust code.
pub fn emit_rust_dsl(diagram: &Diagram) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "iso_diagram! {{");
    let _ = writeln!(out, "    title: {:?},", diagram.title);
    let _ = writeln!(out, "    theme: {:?},", diagram.theme);

    let mut node_vars: HashMap<usize, String> = HashMap::new();
    let mut used_vars: HashMap<String, usize> = HashMap::new();
    for node in &diagram.nodes {
        let base = crate::model::sanitize_slug(&node.slug);
        let count = used_vars.entry(base.clone()).or_insert(0);
        let unique = if *count == 0 {
            base
        } else {
            format!("{base}_{}", *count)
        };
        *count += 1;
        node_vars.insert(node.id.0, unique);
    }

    for node in diagram.nodes.iter().filter(|n| n.zone.is_none()) {
        write_node(&mut out, "    ", node, &node_vars[&node.id.0]);
    }

    for zone in diagram.zones.iter().filter(|z| z.parent.is_none()) {
        write_zone(&mut out, "    ", diagram, zone.id, &node_vars);
    }

    if !diagram.edges.is_empty() {
        let _ = writeln!(out, "    edges {{");
        for edge in &diagram.edges {
            let from_var = &node_vars[&edge.from.0];
            let to_var = &node_vars[&edge.to.0];
            let op = if edge.bidirectional {
                "<->"
            } else {
                match edge.style {
                    EdgeStyle::Solid => "->",
                    EdgeStyle::Dashed => "~>",
                    EdgeStyle::Dotted => "..>",
                }
            };
            let label_part = edge
                .label
                .as_ref()
                .map(|l| format!(" : {l:?}"))
                .unwrap_or_default();
            let _ = writeln!(
                out,
                "        {from_var} {op} {to_var} [{:?}]{label_part};",
                edge.tint
            );
        }
        let _ = writeln!(out, "    }}");
    }

    let _ = writeln!(out, "}}");
    out
}

fn write_zone(
    out: &mut String,
    indent: &str,
    diagram: &Diagram,
    zone_id: ZoneId,
    node_vars: &HashMap<usize, String>,
) {
    let z = diagram.zone(zone_id);
    let zvar = crate::model::sanitize_slug(&z.slug);
    let _ = writeln!(
        out,
        "{indent}zone {zvar} ({:?}, {:?}, {:?}) {{",
        z.title, z.style, z.tint
    );
    let inner = format!("{indent}    ");
    for child in diagram.zones.iter().filter(|cz| cz.parent == Some(zone_id)) {
        write_zone(out, &inner, diagram, child.id, node_vars);
    }
    for node in diagram.nodes.iter().filter(|n| n.zone == Some(zone_id)) {
        write_node(out, &inner, node, &node_vars[&node.id.0]);
    }
    let _ = writeln!(out, "{indent}}}");
}

fn write_node(out: &mut String, indent: &str, node: &crate::model::NodeSpec, var: &str) {
    let sub = node
        .subtitle
        .as_ref()
        .map(|s| format!(", {s:?}"))
        .unwrap_or_default();
    let mut attrs = Vec::new();
    if let Some(t) = node.tint {
        attrs.push(format!("{t:?}"));
    }
    if node.status != NodeStatus::Healthy {
        attrs.push(format!("{:?}", node.status));
    }
    let attr_str = if attrs.is_empty() {
        String::new()
    } else {
        format!(" [{}]", attrs.join(", "))
    };
    let _ = writeln!(
        out,
        "{indent}node {var} = {} => ({:?}{sub}){attr_str};",
        node.asset.rust_expr(),
        node.title,
    );
}
