//! The Contacts data source: owns the actual contact records as a JSON
//! data block and declares the actions ("add contact", "remove contact")
//! that mutate it. This is a `Source` app in the source/manipulator/sink
//! model from root `CLAUDE.md` — it's the only app allowed to write this
//! data (enforced by the kernel, see `os::kernel::data`); everything else
//! (the `contacts_sort`/`contacts_filter` manipulators, the `contacts`
//! sink) only ever reads it.
//!
//! Launched directly, it renders the generic [`crate::app::source_inspector::SourceInspector`]
//! rather than a bespoke UI — deliberately, since a source having no
//! opinion about presentation is the point of the architecture.

use std::collections::HashMap;

use leptos::prelude::*;
use serde_json::json;

use super::source_inspector::SourceInspector;
use super::{Action, App, AppRef};
use crate::os::kernel::{AppRole, BlockId, FieldKind, Pid, SettingField, SettingValue, SizeConstraints};
use crate::os::kernel::{self as kernel};

/// Public so other Contacts apps (the sink, the manipulators) can find
/// this source by id without a hardcoded string.
pub const ID: &str = "contacts-source";

const ACTIONS: &[Action] = &[
    Action {
        id: "add_contact",
        label: "Add contact",
        params: &[
            SettingField {
                key: "name",
                label: "Name",
                kind: FieldKind::Text { placeholder: "Name" },
                default: SettingValue::Text(String::new()),
            },
            SettingField {
                key: "email",
                label: "Email",
                kind: FieldKind::Text { placeholder: "Email" },
                default: SettingValue::Text(String::new()),
            },
        ],
    },
    Action {
        id: "remove_contact",
        label: "Remove contact",
        params: &[SettingField {
            key: "id",
            label: "Contact ID",
            kind: FieldKind::Number { min: 0.0, max: 1_000_000.0, step: 1.0, suffix: "" },
            default: SettingValue::Number(0.0),
        }],
    },
];

pub struct ContactsSourceApp;

impl App for ContactsSourceApp {
    fn id(&self) -> &'static str {
        ID
    }
    fn title(&self) -> &'static str {
        "Contacts Source"
    }
    fn description(&self) -> &'static str {
        "Owns the contact records other apps read through a pipeline."
    }
    fn icon(&self) -> &'static str {
        "\u{1F5C3}\u{FE0F}"
    }
    fn is_singleton(&self) -> bool {
        true
    }
    fn preferred_size(&self) -> (f64, f64) {
        (480.0, 360.0)
    }
    fn size_constraints(&self) -> SizeConstraints {
        SizeConstraints {
            min_width: 380.0,
            min_height: 280.0,
            max_width: None,
            max_height: None,
            aspect_ratio: None,
        }
    }
    fn role(&self) -> Option<AppRole> {
        Some(AppRole::Source)
    }
    fn seed(&self) -> serde_json::Value {
        json!([
            {"id": 1, "name": "Ada Lovelace", "role": "Systems Architect", "email": "ada@aquos.dev", "phone": "+1 555-0142"},
            {"id": 2, "name": "Grace Hopper", "role": "Compiler Engineer", "email": "grace@aquos.dev", "phone": "+1 555-0187"},
            {"id": 3, "name": "Alan Turing", "role": "Kernel Maintainer", "email": "alan@aquos.dev", "phone": "+1 555-0113"},
        ])
    }
    fn schema(&self) -> serde_json::Value {
        json!(["id", "name", "role", "email", "phone"])
    }
    fn actions(&self) -> &'static [Action] {
        ACTIONS
    }
    fn invoke_action(&self, block_id: BlockId, owner: Pid, action_id: &str, params: &HashMap<&'static str, SettingValue>) {
        let Some(block) = kernel::read_block(block_id) else {
            return;
        };
        let mut records: Vec<serde_json::Value> = block.data.as_array().cloned().unwrap_or_default();
        match action_id {
            "add_contact" => {
                let name = text_param(params, "name");
                if name.trim().is_empty() {
                    return;
                }
                let email = text_param(params, "email");
                let next_id = records.iter().filter_map(|r| r.get("id").and_then(|v| v.as_u64())).max().unwrap_or(0) + 1;
                records.push(json!({
                    "id": next_id,
                    "name": name,
                    "role": "New contact",
                    "email": email,
                    "phone": "\u{2014}",
                }));
            }
            "remove_contact" => {
                let id = number_param(params, "id") as u64;
                records.retain(|r| r.get("id").and_then(|v| v.as_u64()) != Some(id));
            }
            _ => {}
        }
        kernel::update_block(block_id, owner, serde_json::Value::Array(records));
    }
    fn render(&self, pid: Pid) -> AnyView {
        view! { <SourceInspector source=AppRef(&ContactsSourceApp) pid=pid /> }.into_any()
    }
}

fn text_param(params: &HashMap<&'static str, SettingValue>, key: &str) -> String {
    match params.get(key) {
        Some(SettingValue::Text(t)) => t.clone(),
        _ => String::new(),
    }
}

fn number_param(params: &HashMap<&'static str, SettingValue>, key: &str) -> f64 {
    match params.get(key) {
        Some(SettingValue::Number(n)) => *n,
        Some(SettingValue::Text(t)) => t.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}
