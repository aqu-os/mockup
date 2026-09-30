//! Whole-desktop fullscreen, backed by the browser Fullscreen API.

use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{window, Event};

#[derive(Clone, Copy)]
pub struct FullscreenState(pub RwSignal<bool>);

/// Installs fullscreen state into the context tree and keeps it in sync
/// with the browser — e.g. when the user exits via Esc rather than our own
/// toggle button.
pub fn provide() {
    let state = RwSignal::new(false);
    provide_context(FullscreenState(state));

    if let Some(document) = window().and_then(|w| w.document()) {
        let sync: Closure<dyn Fn(Event)> = Closure::new(move |_: Event| {
            let is_fullscreen = window()
                .and_then(|w| w.document())
                .is_some_and(|d| d.fullscreen_element().is_some());
            state.set(is_fullscreen);
        });
        let _ = document
            .add_event_listener_with_callback("fullscreenchange", sync.as_ref().unchecked_ref());
        sync.forget();
    }
}

/// Enters fullscreen if the document isn't already in it, otherwise exits.
pub fn toggle() {
    let Some(document) = window().and_then(|w| w.document()) else {
        return;
    };
    if document.fullscreen_element().is_some() {
        document.exit_fullscreen();
    } else if let Some(root) = document.document_element() {
        let _ = root.request_fullscreen();
    }
}
