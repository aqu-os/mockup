//! Generic window chrome: title bar with per-manager action buttons and
//! close, drag-to-move (managers that support it), drag-to-resize via edge
//! and corner handles (managers that support it), and focus. Knows
//! nothing about what an app renders inside it — see `app::App::render`
//! for that — and nothing about *how* a manager arranges windows beyond
//! the [`Placement`] it hands back each render.

use leptos::ev::PointerEvent;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use crate::app::{AppRef, Apps};
use crate::os::kernel::WindowId;
use crate::os::window_manager::{Extent, Placement, ResizeEdge, WindowManager as _};
use crate::os::workspace::Workspace;

/// The concrete CSS a [`Placement`] translates to. Every field is always
/// set (rather than conditionally including attributes per variant) so
/// switching a workspace's layout at runtime can't leave stale absolute
/// positioning or flex sizing behind from a previous mode.
#[derive(Clone, Debug, PartialEq)]
struct ChromeStyle {
    position: &'static str,
    left: String,
    top: String,
    width: String,
    height: String,
    z_index: String,
    flex_grow: String,
    flex_shrink: String,
    flex_basis: String,
}

fn style_for(placement: Option<Placement>) -> ChromeStyle {
    match placement {
        Some(Placement::Absolute { x, y, width, height, z }) => ChromeStyle {
            position: "absolute",
            left: format!("{x}px"),
            top: format!("{y}px"),
            width: format!("{width}px"),
            height: format!("{height}px"),
            z_index: z.to_string(),
            flex_grow: "0".into(),
            flex_shrink: "0".into(),
            flex_basis: "auto".into(),
        },
        Some(Placement::Flex { grow, height }) => ChromeStyle {
            position: "relative",
            left: "auto".into(),
            top: "auto".into(),
            width: "auto".into(),
            height: match height {
                Extent::Fixed(px) => format!("{px}px"),
                Extent::Fill => "100%".into(),
            },
            z_index: "0".into(),
            flex_grow: grow.to_string(),
            flex_shrink: "1".into(),
            flex_basis: "0%".into(),
        },
        Some(Placement::Scroll { width, height }) => {
            let extent = |e: Extent| match e {
                Extent::Fixed(px) => format!("{px}px"),
                Extent::Fill => "100%".into(),
            };
            let width = extent(width);
            ChromeStyle {
                position: "relative",
                left: "auto".into(),
                top: "auto".into(),
                width: width.clone(),
                height: extent(height),
                z_index: "0".into(),
                flex_grow: "0".into(),
                flex_shrink: "0".into(),
                flex_basis: width,
            }
        }
        None => ChromeStyle {
            position: "absolute",
            left: "0px".into(),
            top: "0px".into(),
            width: "0px".into(),
            height: "0px".into(),
            z_index: "0".into(),
            flex_grow: "0".into(),
            flex_shrink: "0".into(),
            flex_basis: "auto".into(),
        },
    }
}

#[component]
pub fn WindowFrame(workspace: Workspace, id: WindowId, kind: AppRef) -> impl IntoView {
    let apps = expect_context::<Apps>();
    let workspace_id = workspace.id;

    let is_focused = Signal::derive(move || workspace.focused() == Some(id));
    let is_maximized = move || workspace.active_manager().is_maximized(id);
    let is_minimized = move || workspace.active_manager().is_minimized(id);
    let supports_drag = move || workspace.active_manager().supports_drag();

    let chrome = Memo::new(move |_| style_for(workspace.active_manager().placement(id)));
    let actions = Memo::new(move |_| workspace.active_manager().actions(id));

    // Title, icon and the app's own body are fixed for the lifetime of a
    // window, so they're read/built exactly once here, outside any
    // reactive closure. If they lived inside a closure that re-runs on
    // every layout change, the app's content would be torn down and
    // rebuilt on each of those — silently wiping local state like typed
    // notes or the file explorer's current folder, and even swallowing
    // clicks whose target node got replaced mid-event.
    let meta = workspace.window_meta(id).expect("window exists when frame is created");
    let title = meta.title.clone();
    let icon = meta.icon;
    let body = kind.render(meta.owner);

    let drag_offset: RwSignal<Option<(f64, f64)>> = RwSignal::new(None);

    let start_drag = move |ev: PointerEvent| {
        workspace.focus(id);
        let manager = workspace.active_manager();
        if !manager.supports_drag() {
            return;
        }
        if let Some(Placement::Absolute { x, y, .. }) = manager.placement(id) {
            drag_offset.set(Some((ev.client_x() as f64 - x, ev.client_y() as f64 - y)));
        }
        if let Some(target) = ev.target().and_then(|t| t.dyn_into::<HtmlElement>().ok()) {
            let _ = target.set_pointer_capture(ev.pointer_id());
        }
    };

    let drag_move = move |ev: PointerEvent| {
        if let Some((ox, oy)) = drag_offset.get_untracked() {
            workspace
                .active_manager()
                .drag_to(id, ev.client_x() as f64 - ox, ev.client_y() as f64 - oy);
        }
    };

    let end_drag = move |_: PointerEvent| drag_offset.set(None);

    let resizable_edges = Memo::new(move |_| workspace.active_manager().resizable_edges(id).to_vec());

    // The pointer's last seen position while dragging a resize handle, so
    // each move event can hand the manager a delta since last time rather
    // than an absolute position — the manager needs a delta to know which
    // side is the anchor that shouldn't move.
    let resize_from: RwSignal<Option<(ResizeEdge, f64, f64)>> = RwSignal::new(None);

    let start_resize = move |edge: ResizeEdge| {
        move |ev: PointerEvent| {
            ev.stop_propagation();
            workspace.focus(id);
            resize_from.set(Some((edge, ev.client_x() as f64, ev.client_y() as f64)));
            if let Some(target) = ev.target().and_then(|t| t.dyn_into::<HtmlElement>().ok()) {
                let _ = target.set_pointer_capture(ev.pointer_id());
            }
        }
    };

    let resize_move = move |ev: PointerEvent| {
        if let Some((edge, lx, ly)) = resize_from.get_untracked() {
            let (x, y) = (ev.client_x() as f64, ev.client_y() as f64);
            workspace.active_manager().resize_by(id, edge, x - lx, y - ly);
            resize_from.set(Some((edge, x, y)));
        }
    };

    let end_resize = move |_: PointerEvent| resize_from.set(None);

    view! {
        <div
            class="window"
            class:window--focused=is_focused
            class:window--maximized=is_maximized
            style:display=move || if is_minimized() { "none" } else { "flex" }
            style:position=move || chrome.get().position
            style:left=move || chrome.get().left
            style:top=move || chrome.get().top
            style:width=move || chrome.get().width
            style:height=move || chrome.get().height
            style:z-index=move || chrome.get().z_index
            style:flex-grow=move || chrome.get().flex_grow
            style:flex-shrink=move || chrome.get().flex_shrink
            style:flex-basis=move || chrome.get().flex_basis
            on:pointerdown=move |_| workspace.focus(id)
        >
            <div
                class="window__titlebar"
                class:window__titlebar--static=move || !supports_drag()
                on:pointerdown=start_drag
                on:pointermove=drag_move
                on:pointerup=end_drag
            >
                <span class="window__icon">{icon}</span>
                <span class="window__title">{title}</span>
                <div class="window__actions">
                    <For each=move || actions.get() key=|a| a.kind let(action)>
                        {
                            let action_kind = action.kind;
                            view! {
                                <button
                                    class="window__action"
                                    title=action.label
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        workspace.active_manager().perform(id, action_kind);
                                    }
                                >
                                    {action.icon}
                                </button>
                            }
                        }
                    </For>
                </div>
                <button
                    class="window__close"
                    on:click=move |ev| {
                        ev.stop_propagation();
                        apps.close(workspace_id, id)
                    }
                >
                    "\u{2715}"
                </button>
            </div>
            <div class="window__body">{body}</div>
            <For each=move || resizable_edges.get() key=|e| *e let(edge)>
                {
                    let on_down = start_resize(edge);
                    view! {
                        <div
                            class=format!("window__resize-handle window__resize-handle--{}", edge.css_class())
                            style:cursor=edge.cursor()
                            on:pointerdown=on_down
                            on:pointermove=resize_move
                            on:pointerup=end_resize
                        ></div>
                    }
                }
            </For>
        </div>
    }
}
