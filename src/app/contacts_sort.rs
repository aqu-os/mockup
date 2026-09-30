//! Sorts contact records by name or role — a `Manipulator`: read-only
//! over whatever it's handed, produces a new derived value, never touches
//! the source's block (see root `CLAUDE.md`). Its sort field is a
//! per-pipeline-instance parameter (`os::kernel::Owner::Process`), not a
//! persisted app setting, since two Sort steps in different pipelines
//! shouldn't share a choice.
//!
//! Manipulators are never launched directly — they're filtered out of the
//! start menu (see `ui::menu_bar`) and only ever run headless as part of
//! a pipeline (`app::pipeline::add_manipulator`) — so [`App::render`]
//! here is only a formality.

use std::collections::HashMap;

use leptos::prelude::*;

use super::App;
use crate::os::kernel::{AppRole, FieldKind, Pid, SettingField, SettingValue, SizeConstraints};

pub const ID: &str = "contacts-sort";

const SORT_OPTIONS: &[&str] = &["Name", "Role"];

const PARAMS: &[SettingField] = &[SettingField {
    key: "sort_by",
    label: "Sort by",
    kind: FieldKind::Choice(SORT_OPTIONS),
    default: SettingValue::Choice(0),
}];

pub struct ContactsSortApp;

impl App for ContactsSortApp {
    fn id(&self) -> &'static str {
        ID
    }
    fn title(&self) -> &'static str {
        "Sort"
    }
    fn description(&self) -> &'static str {
        "Sorts contacts by name or role."
    }
    fn icon(&self) -> &'static str {
        "\u{2195}\u{FE0F}"
    }
    fn preferred_size(&self) -> (f64, f64) {
        (300.0, 200.0)
    }
    fn size_constraints(&self) -> SizeConstraints {
        SizeConstraints {
            min_width: 200.0,
            min_height: 150.0,
            max_width: None,
            max_height: None,
            aspect_ratio: None,
        }
    }
    fn role(&self) -> Option<AppRole> {
        Some(AppRole::Manipulator)
    }
    fn param_schema(&self) -> &'static [SettingField] {
        PARAMS
    }
    fn apply(&self, input: serde_json::Value, params: &HashMap<&'static str, SettingValue>) -> serde_json::Value {
        let sort_by_role = matches!(params.get("sort_by"), Some(SettingValue::Choice(1)));
        let mut records = input.as_array().cloned().unwrap_or_default();
        records.sort_by(|a, b| {
            let key = if sort_by_role { "role" } else { "name" };
            let a = a.get(key).and_then(|v| v.as_str()).unwrap_or("");
            let b = b.get(key).and_then(|v| v.as_str()).unwrap_or("");
            a.cmp(b)
        });
        serde_json::Value::Array(records)
    }
    fn render(&self, _pid: Pid) -> AnyView {
        view! { <div class="manipulator-placeholder">"This app has no window."</div> }.into_any()
    }
}
