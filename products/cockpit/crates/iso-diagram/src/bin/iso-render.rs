#![forbid(unsafe_code)]
//! Standalone zero-dep SVG CLI: `iso-render --scene <name> [--theme paper|obsidian] [out.svg]`.

use std::fs;
use std::process;

use iso_diagram::catalog;
use iso_diagram::model::ThemePreset;
use iso_diagram::svg::render_svg;

fn main() {
    let mut args = std::env::args().skip(1);
    let mut scene = String::from("campus-ha");
    let mut theme = ThemePreset::WarmPaper;
    let mut out_path: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scene" => {
                if let Some(v) = args.next() {
                    scene = v;
                }
            }
            "--theme" => {
                if let Some(v) = args.next() {
                    theme = if v == "obsidian" {
                        ThemePreset::ObsidianSlate
                    } else {
                        ThemePreset::WarmPaper
                    };
                }
            }
            "--help" | "-h" => {
                eprintln!(
                    "Usage: iso-render --scene <campus-ha|payment-gateway|asset-gallery> [--theme paper|obsidian] [output.svg]"
                );
                process::exit(0);
            }
            other if !other.starts_with('-') && out_path.is_none() => {
                out_path = Some(other.to_string());
            }
            other => {
                eprintln!("Unknown argument: {other}");
                process::exit(2);
            }
        }
    }

    let Some(mut diagram) = catalog::by_name(&scene) else {
        eprintln!("Unknown scene `{scene}`; expected campus-ha, payment-gateway, or asset-gallery");
        process::exit(2);
    };
    diagram.theme = theme;
    let svg = render_svg(&diagram, 1440.0, 900.0);

    match out_path {
        Some(path) => {
            if let Err(err) = fs::write(&path, svg) {
                eprintln!("Failed to write `{path}`: {err}");
                process::exit(1);
            }
            println!("Wrote {path}");
        }
        None => print!("{svg}"),
    }
}
