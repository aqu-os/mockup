//! Notes: a trivial scratchpad app. Its only real purpose in the mockup is
//! to be a second, non-singleton app — so a workspace can hold more than
//! one window at a time, which is what actually makes the tiling and
//! infinite-scroll layouts visible.

use leptos::prelude::*;

use super::App;
use crate::os::kernel::{self as kernel, FieldKind, Owner, Pid, SettingField, SettingValue, SizeConstraints};

const ID: &str = "notes";
const FONT_SIZE_KEY: &str = "font_size";
const FONT_SIZES: &[&str] = &["Small", "Medium", "Large"];
const FONT_SIZE_CSS: &[&str] = &["0.75rem", "0.85rem", "1rem"];

const SETTINGS: &[SettingField] = &[SettingField {
    key: FONT_SIZE_KEY,
    label: "Font size",
    kind: FieldKind::Choice(FONT_SIZES),
    default: SettingValue::Choice(1),
}];

pub struct NotesApp;

impl App for NotesApp {
    fn id(&self) -> &'static str {
        ID
    }
    fn title(&self) -> &'static str {
        "Notes"
    }
    fn description(&self) -> &'static str {
        "A simple scratchpad for quick notes."
    }
    fn icon(&self) -> &'static str {
        "\u{1F4DD}"
    }
    fn preferred_size(&self) -> (f64, f64) {
        (320.0, 260.0)
    }
    fn size_constraints(&self) -> SizeConstraints {
        SizeConstraints {
            min_width: 220.0,
            min_height: 160.0,
            max_width: None,
            max_height: None,
            aspect_ratio: None,
        }
    }
    fn settings_manifest(&self) -> &'static [SettingField] {
        SETTINGS
    }
    fn render(&self, _pid: Pid) -> AnyView {
        view! { <Notes /> }.into_any()
    }
}

#[component]
fn Notes() -> impl IntoView {
    let text = RwSignal::new(String::new());

    let font_size = move || {
        let choice = kernel::get_setting_choice(Owner::App(ID), FONT_SIZE_KEY, 1);
        FONT_SIZE_CSS.get(choice).copied().unwrap_or(FONT_SIZE_CSS[1])
    };

    view! {
        <textarea
            class="notes__area"
            placeholder="Jot something down..."
            style:font-size=font_size
            prop:value=move || text.get()
            on:input=move |ev| text.set(event_target_value(&ev))
        ></textarea>
    }
}
