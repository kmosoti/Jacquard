//! Integration tests for `telemetry-scene` JSON schemas and 2.5D scene projection.

use iso_diagram::layout::compile_layout;
use iso_diagram::model::{NodeStatus, ThemePreset};
use iso_diagram::svg::render_svg;
use telemetry_scene::{
    cluster_to_diagram, example_cluster_bundle, example_cluster_topology, parse_cluster_json,
    parse_telemetry_json, telemetry_to_diagram, telemetry_to_diagram_themed,
};

#[test]
fn telemetry_bundle_json_roundtrip_and_2d5_projection_succeed() {
    let bundle = example_cluster_bundle();
    let json = serde_json::to_string(&bundle).expect("serialize bundle");
    let decoded = parse_telemetry_json(&json).expect("deserialize bundle");
    assert_eq!(decoded.snapshot, bundle.snapshot);
    assert_eq!(decoded.custody, bundle.custody);
    assert_eq!(decoded.metric_series.len(), bundle.metric_series.len());
    for ((lt, lv), (rt, rv)) in decoded
        .metric_series
        .iter()
        .zip(bundle.metric_series.iter())
    {
        assert_eq!(lt, rt);
        assert!((lv - rv).abs() < 1e-3, "metric value drift: {lv} vs {rv}");
    }

    let diag = telemetry_to_diagram(&decoded);
    assert!(diag.title.contains("example-node-01"));
    assert!(diag.zones.len() >= 3);
    assert!(diag.nodes.len() >= 8);

    let layout = compile_layout(&diag);
    assert_eq!(layout.nodes.len(), diag.nodes.len());
    for edge in &layout.edges {
        for seg in edge.points.windows(2) {
            let du = (seg[1].0 - seg[0].0).abs();
            let dv = (seg[1].1 - seg[0].1).abs();
            assert!((du < 1e-4) ^ (dv < 1e-4));
        }
    }

    let svg = render_svg(&diag, 1280.0, 800.0);
    assert!(svg.starts_with("<svg "));
    assert!(svg.contains("telemetry-agent"));
}

#[test]
fn custody_lag_marks_wal_custody_node_degraded() {
    let mut bundle = example_cluster_bundle();
    if let Some(custody) = &mut bundle.custody {
        custody.highest_record_id = 145;
        custody.canonical_through = 130;
    }

    let diag = telemetry_to_diagram_themed(&bundle, ThemePreset::ObsidianSlate);
    let nid = diag
        .find_node_by_slug("wal_custody")
        .expect("wal_custody node exists");
    assert_eq!(diag.node(nid).status, NodeStatus::Degraded);
}

#[test]
fn cluster_topology_json_projects_and_renders() {
    let cluster = example_cluster_topology();
    let json = serde_json::to_string(&cluster).expect("serialize cluster");
    let decoded = parse_cluster_json(&json).expect("deserialize cluster");
    let diag = cluster_to_diagram(&decoded, ThemePreset::WarmPaper);
    assert_eq!(diag.nodes.len(), 5);
    assert_eq!(diag.edges.len(), 5);
    let degraded = diag
        .find_node_by_slug("worker_02")
        .expect("worker_02 node exists");
    assert_eq!(diag.node(degraded).status, NodeStatus::Degraded);
}
