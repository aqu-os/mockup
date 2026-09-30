//! Settings: reads every installed app's settings manifest (and the OS's
//! own, for Desktop/Window Managers/About — manifests owned by this module,
//! see [`desktop_settings`], [`window_manager_settings`], [`about_settings`]
//! below) and renders a form for whichever one is selected. Doesn't know
//! anything about what a field *means* — it just renders a control
//! appropriate for the field's kind and reads/writes through the kernel's
//! settings store.

use leptos::prelude::*;

use super::{App, AppRef, Apps};
use crate::os::kernel::{self as kernel, os_category, FieldKind, Owner, Pid, SettingField, SettingValue, SizeConstraints};

/// Manifest for the Desktop category — read by `ui::desktop` to pick a
/// wallpaper. Lives here, not in the kernel, purely for colocation with
/// the other OS setting manifests below; it has no non-kernel
/// dependencies of its own.
pub mod desktop_settings {
    use super::{FieldKind, SettingField, SettingValue};

    pub const WALLPAPER_KEY: &str = "wallpaper";
    pub const WALLPAPER_OPTIONS: &[&str] =
        &["Mountains (Dusk)", "Mountains (Warm)", "Solid Gradient"];

    pub const FIELDS: &[SettingField] = &[SettingField {
        key: WALLPAPER_KEY,
        label: "Wallpaper",
        kind: FieldKind::Choice(WALLPAPER_OPTIONS),
        default: SettingValue::Choice(0),
    }];
}

/// Manifest for the Window Managers category — the tiling and
/// infinite-scroll managers read their resize step through the same keys.
/// This one *can't* live in the kernel: it needs `tiling`/`infinite_scroll`
/// (non-kernel `os::managers` policy), and the kernel isn't allowed to
/// depend upward on them.
pub mod window_manager_settings {
    use super::{FieldKind, SettingField, SettingValue};
    use crate::os::managers::{infinite_scroll, tiling};

    pub const FIELDS: &[SettingField] = &[
        SettingField {
            key: tiling::STEP_KEY,
            label: "Tiling resize step",
            kind: FieldKind::Number { min: 0.05, max: 0.25, step: 0.05, suffix: "" },
            default: SettingValue::Number(tiling::DEFAULT_STEP),
        },
        SettingField {
            key: infinite_scroll::STEP_KEY,
            label: "Infinite-scroll resize step",
            kind: FieldKind::Number { min: 20.0, max: 120.0, step: 20.0, suffix: "px" },
            default: SettingValue::Number(infinite_scroll::DEFAULT_STEP),
        },
    ];
}

/// Manifest for the About category — read by `ui::menu_bar` for the clock
/// format.
pub mod about_settings {
    use super::{FieldKind, SettingField, SettingValue};

    pub const CLOCK_SECONDS_KEY: &str = "clock_seconds";

    pub const FIELDS: &[SettingField] = &[SettingField {
        key: CLOCK_SECONDS_KEY,
        label: "Show seconds in the clock",
        kind: FieldKind::Toggle,
        default: SettingValue::Bool(true),
    }];
}

pub struct SettingsManagerApp;

impl App for SettingsManagerApp {
    fn id(&self) -> &'static str {
        "settings"
    }
    fn title(&self) -> &'static str {
        "Settings"
    }
    fn description(&self) -> &'static str {
        "Configure the system and your apps."
    }
    fn icon(&self) -> &'static str {
        "\u{2699}\u{FE0F}"
    }
    fn is_system(&self) -> bool {
        true
    }
    fn is_singleton(&self) -> bool {
        true
    }
    fn preferred_size(&self) -> (f64, f64) {
        (620.0, 440.0)
    }
    fn size_constraints(&self) -> SizeConstraints {
        SizeConstraints {
            min_width: 480.0,
            min_height: 340.0,
            max_width: None,
            max_height: None,
            aspect_ratio: None,
        }
    }
    fn render(&self, _pid: Pid) -> AnyView {
        view! { <SettingsManager /> }.into_any()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Selected {
    Os(&'static str),
    App(AppRef),
}

#[component]
fn SettingsManager() -> impl IntoView {
    let apps = expect_context::<Apps>();
    let installed = apps.installed();

    let selected = RwSignal::new(Selected::Os(os_category::DESKTOP));

    let app_categories = move || {
        let installed_set = installed.all().get();
        super::ALL
            .iter()
            .copied()
            .filter(|k| installed_set.contains(k) && !k.settings_manifest().is_empty())
            .collect::<Vec<_>>()
    };

    view! {
        <div class="settings">
            <div class="settings__sidebar">
                <div class="settings__section-label">"System"</div>
                {os_category::ALL
                    .iter()
                    .map(|(id, label)| {
                        let id = *id;
                        let is_active = move || selected.get() == Selected::Os(id);
                        view! {
                            <button
                                class="settings__nav-item"
                                class:is-active=is_active
                                on:click=move |_| selected.set(Selected::Os(id))
                            >
                                {*label}
                            </button>
                        }
                    })
                    .collect_view()}

                <div class="settings__section-label">"Apps"</div>
                <For each=app_categories key=|k| *k let(kind)>
                    {
                        let is_active = move || selected.get() == Selected::App(kind);
                        view! {
                            <button
                                class="settings__nav-item"
                                class:is-active=is_active
                                on:click=move |_| selected.set(Selected::App(kind))
                            >
                                <span class="settings__nav-icon">{kind.icon()}</span>
                                {kind.title()}
                            </button>
                        }
                    }
                </For>
            </div>

            <div class="settings__panel">
                {move || match selected.get() {
                    Selected::Os(id) => {
                        let (label, fields): (&str, &[SettingField]) = match id {
                            os_category::DESKTOP => ("Desktop", desktop_settings::FIELDS),
                            os_category::WINDOW_MANAGERS => {
                                ("Window Managers", window_manager_settings::FIELDS)
                            }
                            os_category::ABOUT => ("About", about_settings::FIELDS),
                            _ => ("", &[]),
                        };
                        view! {
                            <div class="settings__panel-header">
                                <div class="settings__panel-title">{label}</div>
                            </div>
                            {render_fields(Owner::Os(id), fields)}
                            {(id == os_category::ABOUT)
                                .then(|| {
                                    view! {
                                        <div class="settings__about">
                                            "Aquos — an early-stage OS. This is a UI mockup for trying out desktop ideas, not the real kernel."
                                        </div>
                                    }
                                })}
                        }
                            .into_any()
                    }
                    Selected::App(kind) => {
                        if !installed.is_installed(kind) {
                            view! {
                                <div class="settings__empty">"This app is no longer installed."</div>
                            }
                                .into_any()
                        } else {
                            view! {
                                <div class="settings__panel-header">
                                    <span class="settings__panel-icon">{kind.icon()}</span>
                                    <div>
                                        <div class="settings__panel-title">{kind.title()}</div>
                                        <div class="settings__panel-desc">{kind.description()}</div>
                                    </div>
                                </div>
                                {render_fields(Owner::App(kind.id()), kind.settings_manifest())}
                            }
                                .into_any()
                        }
                    }
                }}
            </div>
        </div>
    }
}

/// Renders a settings form for `fields`, reading/writing through the
/// kernel's settings store under `owner`. Generic over any `SettingField`
/// list, not just app/OS manifests — also reused by `app::pipeline` to
/// render a manipulator's per-instance parameters (stored under
/// `Owner::Process`), since both are "a form generically driven by a
/// declared field list, persisted in the kernel settings store".
pub(crate) fn render_fields(owner: Owner, fields: &'static [SettingField]) -> impl IntoView {
    if fields.is_empty() {
        return view! { <div class="settings__empty">"This app has no settings."</div> }.into_any();
    }
    fields
        .iter()
        .map(|field| {
            let field = field.clone();
            view! {
                <div class="settings-field">
                    <div class="settings-field__label">{field.label}</div>
                    <div class="settings-field__control">{render_control(owner, field)}</div>
                </div>
            }
        })
        .collect_view()
        .into_any()
}

fn render_control(owner: Owner, field: SettingField) -> impl IntoView {
    match field.kind {
        FieldKind::Toggle => {
            let default = matches!(field.default, SettingValue::Bool(true));
            let checked = move || kernel::get_setting_bool(owner, field.key, default);
            view! {
                <label class="settings-toggle">
                    <input
                        type="checkbox"
                        prop:checked=checked
                        on:change=move |ev| {
                            kernel::set_setting(owner, field.key, SettingValue::Bool(event_target_checked(&ev)))
                        }
                    />
                    <span class="settings-toggle__track">
                        <span class="settings-toggle__thumb"></span>
                    </span>
                </label>
            }
                .into_any()
        }
        FieldKind::Number { min, max, step, suffix } => {
            let default = match field.default {
                SettingValue::Number(n) => n,
                _ => min,
            };
            let value = move || kernel::get_setting_number(owner, field.key, default);
            view! {
                <div class="settings-number">
                    <input
                        type="range"
                        min=min.to_string()
                        max=max.to_string()
                        step=step.to_string()
                        prop:value=move || value().to_string()
                        on:input=move |ev| {
                            let v: f64 = event_target_value(&ev).parse().unwrap_or(default);
                            kernel::set_setting(owner, field.key, SettingValue::Number(v));
                        }
                    />
                    <span class="settings-number__value">
                        {move || format!("{}{}", format_number(value()), suffix)}
                    </span>
                </div>
            }
                .into_any()
        }
        FieldKind::Text { placeholder } => {
            let default = match &field.default {
                SettingValue::Text(t) => t.clone(),
                _ => String::new(),
            };
            view! {
                <input
                    class="settings-text"
                    type="text"
                    placeholder=placeholder
                    prop:value=move || kernel::get_setting_text(owner, field.key, &default)
                    on:input=move |ev| {
                        kernel::set_setting(owner, field.key, SettingValue::Text(event_target_value(&ev)))
                    }
                />
            }
                .into_any()
        }
        FieldKind::Choice(options) => {
            let default = match field.default {
                SettingValue::Choice(c) => c,
                _ => 0,
            };
            view! {
                <div class="settings-choice">
                    {options
                        .iter()
                        .enumerate()
                        .map(|(i, label)| {
                            let is_active = move || kernel::get_setting_choice(owner, field.key, default) == i;
                            view! {
                                <button
                                    class="settings-choice__btn"
                                    class:is-active=is_active
                                    on:click=move |_| kernel::set_setting(owner, field.key, SettingValue::Choice(i))
                                >
                                    {*label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            }
                .into_any()
        }
    }
}

fn format_number(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}
