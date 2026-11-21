//! Fine-grained Leptos `0.8.23` `#[island]` for the 2.5D interactive isometric stage.
//!
//! Static SVG markup (`zones`, `routes`, `nodes`, `callouts`) and compact `data-scene`
//! fixed-point geometry are rendered on SSR via `children: Children` (zero `serde_json`,
//! zero `iso-diagram`, zero `core::fmt::float` in `wasm32`). On client orbit, a single
//! reactive `Effect` patches `points`/`x`/`y`/`cx`/`cy`/`viewBox` and topological `<g>`
//! order in place — zero DOM teardown, zero stale index capture.

use leptos::prelude::*;

use crate::views::island_bus::use_island_bus;
#[cfg(target_arch = "wasm32")]
use crate::views::iso_client_math::{ClientProjector, ClientScene};
use crate::views::iso_client_math::{clamp_f32, fmt_t1};
use crate::views::stage_dom::{capture_ptr, clicked_urn, hit_node_target};
#[cfg(target_arch = "wasm32")]
use crate::views::stage_dom::{patch_svg_in_place, read_dom_attr, sync_node_selection};

const STAGE_POPOVER_ID: &str = "stage-actions-popover";
const DEFAULT_YAW: f32 = 45.0;
const DEFAULT_PITCH: f32 = 35.264;
const DEFAULT_ZOOM: f32 = 1.0;

/// Interactive 2.5D isometric orbit/zoom stage island (`#[island]`).
#[island]
pub(crate) fn StageOrbitIsland(
    /// Server-rendered static `<svg id="iso-stage-svg" data-scene="...">` markup.
    children: Children,
) -> impl IntoView {
    let bus = use_island_bus(None);
    let yaw_deg = RwSignal::new(DEFAULT_YAW);
    let pitch_deg = RwSignal::new(DEFAULT_PITCH);
    let zoom = RwSignal::new(DEFAULT_ZOOM);
    let is_2d = RwSignal::new(false);
    let demo_active = RwSignal::new(false);
    let density = RwSignal::new("compact");
    let drag_origin = RwSignal::new(None::<(i32, f32, f32, f32, f32)>);

    #[cfg(target_arch = "wasm32")]
    {
        let scene = ClientScene::decode(&read_dom_attr("iso-stage-svg", "data-scene"));
        if bus.selected_urn.get_untracked().is_none()
            && let Some((first, ..)) = scene.boxes.first()
        {
            bus.selected_urn.set(Some(first.clone()));
        }
        let scene_sv = StoredValue::new(scene);
        Effect::new(move |_| {
            let z = zoom.get();
            let proj = scene_sv.with_value(|s| {
                ClientProjector::fit_orbit(s, yaw_deg.get(), pitch_deg.get(), is_2d.get())
            });
            scene_sv.with_value(|s| patch_svg_in_place(s, proj, z));
        });
        Effect::new(move |_| {
            let sel = bus.selected_urn.get();
            scene_sv.with_value(|s| sync_node_selection(s, sel.as_deref()));
        });
    }

    let on_pointer_down = move |ev: leptos::ev::PointerEvent| {
        if ev.button() == 0 && !hit_node_target(&ev) {
            ev.prevent_default();
            capture_ptr(&ev, true);
            drag_origin.set(Some((
                ev.pointer_id(),
                ev.client_x() as f32,
                ev.client_y() as f32,
                yaw_deg.get_untracked(),
                pitch_deg.get_untracked(),
            )));
        }
    };
    let on_pointer_move = move |ev: leptos::ev::PointerEvent| {
        if let Some((pid, sx, sy, by, bp)) = drag_origin.get_untracked()
            && ev.pointer_id() == pid
        {
            yaw_deg.set((by + ((ev.client_x() as f32) - sx) * 0.42).rem_euclid(360.0));
            pitch_deg.set(clamp_f32(
                bp + ((ev.client_y() as f32) - sy) * 0.16,
                21.2,
                50.8,
            ));
        }
    };
    let on_pointer_up = move |ev: leptos::ev::PointerEvent| {
        if let Some((pid, ..)) = drag_origin.get_untracked()
            && ev.pointer_id() == pid
        {
            capture_ptr(&ev, false);
            drag_origin.set(None);
        }
    };
    let on_click = move |ev: leptos::ev::MouseEvent| {
        if let Some(urn) = clicked_urn(&ev) {
            bus.selected_urn.set(Some(urn));
        }
    };
    let on_wheel = move |ev: leptos::ev::WheelEvent| {
        if ev.ctrl_key() || ev.meta_key() {
            ev.prevent_default();
            let f = if ev.delta_y() > 0.0 { 0.92 } else { 1.08 };
            zoom.update(|z| *z = clamp_f32(*z * f, 0.55, 2.2));
        }
    };
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| match ev.key().as_str() {
        "ArrowLeft" => {
            ev.prevent_default();
            yaw_deg.update(|y| *y = (*y - 15.0).rem_euclid(360.0));
        }
        "ArrowRight" => {
            ev.prevent_default();
            yaw_deg.update(|y| *y = (*y + 15.0).rem_euclid(360.0));
        }
        "ArrowUp" => {
            ev.prevent_default();
            pitch_deg.update(|p| *p = clamp_f32(*p + 4.0, 21.2, 50.8));
        }
        "ArrowDown" => {
            ev.prevent_default();
            pitch_deg.update(|p| *p = clamp_f32(*p - 4.0, 21.2, 50.8));
        }
        "+" | "=" => {
            ev.prevent_default();
            zoom.update(|z| *z = clamp_f32(*z * 1.12, 0.55, 2.2));
        }
        "-" | "_" => {
            ev.prevent_default();
            zoom.update(|z| *z = clamp_f32(*z * 0.89, 0.55, 2.2));
        }
        "0" => {
            ev.prevent_default();
            yaw_deg.set(DEFAULT_YAW);
            pitch_deg.set(DEFAULT_PITCH);
            zoom.set(DEFAULT_ZOOM);
        }
        _ => {}
    };

    view! {
        <figure class="stage-viewport">
            <div
                class="stage-svg-wrap"
                tabindex="0"
                role="region"
                aria-label="Isometric stage viewport (use arrow keys to orbit, plus or minus to zoom, 0 to reset)"
                on:click=on_click
                on:pointerdown=on_pointer_down
                on:pointermove=on_pointer_move
                on:pointerup=on_pointer_up
                on:pointercancel=on_pointer_up
                on:wheel=on_wheel
                on:keydown=on_keydown
                on:contextmenu=move |ev| ev.prevent_default()
            >
                {children()}
            </div>

            <figcaption class="stage-caption num">
                {move || {
                    let y = yaw_deg.get().round() as i32;
                    let p = pitch_deg.get().round() as i32;
                    let z = fmt_t1(zoom.get());
                    let mode = if demo_active.get() { "Demo active" } else { "0 Hz idle" };
                    format!("Orbit {y} deg · Pitch {p} deg · Zoom {z}x · {mode}")
                }}
            </figcaption>

            <div class="stage-toolbar stage-controls" role="toolbar" aria-label="Stage controls">
                <div class="seg-group" role="group" aria-label="Projection mode">
                    <button type="button" class="seg-btn" aria-pressed=move || if !is_2d.get() { "true" } else { "false" } on:click=move |_| is_2d.set(false)>"2.5D"</button>
                    <button type="button" class="seg-btn" aria-pressed=move || if is_2d.get() { "true" } else { "false" } on:click=move |_| is_2d.set(true)>"2D"</button>
                </div>
                <button type="button" class="btn quiet" aria-label="Zoom out" on:click=move |_| zoom.update(|z| *z = clamp_f32(*z * 0.88, 0.55, 2.2))>"-"</button>
                <button type="button" class="btn quiet" aria-label="Zoom in" on:click=move |_| zoom.update(|z| *z = clamp_f32(*z * 1.14, 0.55, 2.2))>"+"</button>
                <button type="button" class="btn quiet" on:click=move |_| { yaw_deg.set(DEFAULT_YAW); pitch_deg.set(DEFAULT_PITCH); zoom.set(DEFAULT_ZOOM); }>"Reset Orbit"</button>
                <button type="button" class="btn stage-popover-trigger" popovertarget=STAGE_POPOVER_ID>"Stage Actions"</button>
                <div id=STAGE_POPOVER_ID popover="auto" class="stage-popover">
                    <button type="button" class="btn quiet" on:click=move |_| demo_active.update(|d| *d = !*d)>
                        {move || if demo_active.get() { "Stop Feature Demo" } else { "Launch Feature Demo" }}
                    </button>
                    <button type="button" class="btn quiet" on:click=move |_| density.update(|d| *d = if *d == "compact" { "spacious" } else { "compact" })>
                        {move || format!("Density: {}", density.get())}
                    </button>
                </div>
            </div>
        </figure>
    }
}
