//! Kernel-private settings storage: a generic key-value store shared by the
//! OS and every app. An app "installs a settings file" by declaring a
//! [`SettingField`] list (its manifest, owned by the app itself — see
//! `crate::app::settings_manager`); this module only stores and retrieves
//! values by `(Owner, key)`, without knowing what any of them mean.
//! Reached only through [`super::syscall`].

use std::collections::HashMap;

use leptos::prelude::*;

use super::process::Pid;

/// Well-known OS settings categories. Both the settings manager (to list
/// them) and whichever OS code actually reads a value (e.g. the tiling
/// manager reading its resize step) need to agree on these ids. Kept here,
/// rather than with the settings *manifests* (which live in
/// `crate::app::settings_manager`), because non-kernel OS code like
/// `os::managers::tiling` needs the category id without depending on the
/// app layer.
pub mod os_category {
    pub const DESKTOP: &str = "desktop";
    pub const WINDOW_MANAGERS: &str = "window-managers";
    pub const ABOUT: &str = "about";

    pub const ALL: &[(&str, &str)] = &[
        (DESKTOP, "Desktop"),
        (WINDOW_MANAGERS, "Window Managers"),
        (ABOUT, "About"),
    ];
}

#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Bool(bool),
    Number(f64),
    Choice(usize),
    Text(String),
}

#[derive(Clone, Copy, Debug)]
pub enum FieldKind {
    Toggle,
    Number { min: f64, max: f64, step: f64, suffix: &'static str },
    Choice(&'static [&'static str]),
    Text { placeholder: &'static str },
}

/// One configurable field in an app's (or OS category's) settings
/// manifest — the "file that describes the app's settings".
#[derive(Clone, Debug)]
pub struct SettingField {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    pub default: SettingValue,
}

/// Who a stored setting belongs to. Apps are identified by their string id
/// rather than the `app::App` trait object so the kernel doesn't have to
/// depend on `app` — settings storage is kernel-level infrastructure,
/// generic over whatever happens to be installed. `Process` scopes a value
/// to one running process instance rather than an app as a whole — used
/// for a manipulator's per-pipeline-instance parameters, since two
/// instances of the same manipulator app (in different pipelines) must not
/// share values.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Owner {
    App(&'static str),
    Os(&'static str),
    Process(Pid),
}

#[derive(Clone, Copy)]
pub(super) struct SettingsState {
    values: RwSignal<HashMap<(Owner, &'static str), SettingValue>>,
}

impl SettingsState {
    pub(super) fn new() -> Self {
        Self {
            values: RwSignal::new(HashMap::new()),
        }
    }

    pub(super) fn get(&self, owner: Owner, key: &'static str) -> Option<SettingValue> {
        self.values.get().get(&(owner, key)).cloned()
    }

    pub(super) fn set(&self, owner: Owner, key: &'static str, value: SettingValue) {
        self.values.update(|m| {
            m.insert((owner, key), value);
        });
    }

    /// Wipes every stored value for `owner` — used when uninstalling an
    /// app with "also remove personalized data" checked.
    pub(super) fn clear_owner(&self, owner: Owner) {
        self.values.update(|m| m.retain(|(o, _), _| *o != owner));
    }
}
