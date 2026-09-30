//! Filters contact records by a name substring — a second `Manipulator`,
//! chained alongside `contacts_sort` to demonstrate that order matters:
//! filtering before sorting vs. sorting before filtering can produce
//! different results. See `contacts_sort` for why this has no real
//! window (never launched directly, only run headless in a pipeline).

use std::collections::HashMap;

use leptos::prelude::*;

use super::App;
use crate::os::kernel::{AppRole, FieldKind, Pid, SettingField, SettingValue, SizeConstraints};

pub const ID: &str = "contacts-filter";

const PARAMS: &[SettingField] = &[SettingField {
    key: "query",
    label: "Search",
    kind: FieldKind::Text { placeholder: "Search name..." },
    default: SettingValue::Text(String::new()),
}];

pub struct ContactsFilterApp;

impl App for ContactsFilterApp {
    fn id(&self) -> &'static str {
        ID
    }
    fn title(&self) -> &'static str {
        "Filter"
    }
    fn description(&self) -> &'static str {
        "Filters contacts by name."
    }
    fn icon(&self) -> &'static str {
        "\u{1F50D}"
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
        let query = match params.get("query") {
            Some(SettingValue::Text(t)) => t.to_lowercase(),
            _ => String::new(),
        };
        if query.is_empty() {
            return input;
        }
        let records = input.as_array().cloned().unwrap_or_default();
        let filtered: Vec<_> = records
            .into_iter()
            .filter(|r| {
                r.get("name")
                    .and_then(|v| v.as_str())
                    .map(|n| n.to_lowercase().contains(&query))
                    .unwrap_or(false)
            })
            .collect();
        serde_json::Value::Array(filtered)
    }
    fn render(&self, _pid: Pid) -> AnyView {
        view! { <div class="manipulator-placeholder">"This app has no window."</div> }.into_any()
    }
}
