//! The bottom menu bar: app launcher, workspace selector, layout switcher,
//! running-task list for the active workspace, fullscreen toggle, and a
//! clock.

use leptos::prelude::*;

use super::fullscreen::{self, FullscreenState};
use super::StartMenuOpen;
use crate::app::settings_manager::about_settings;
use crate::app::Apps;
use crate::os::kernel::{self as kernel, os_category, AppRole, Owner};
use crate::os::window_manager::WindowManager as _;
use crate::os::workspace::{LayoutKind, WorkspaceManager};

#[component]
pub fn MenuBar() -> impl IntoView {
    let apps = expect_context::<Apps>();
    let workspaces = expect_context::<WorkspaceManager>();
    let start_open = expect_context::<StartMenuOpen>().0;
    let is_fullscreen = expect_context::<FullscreenState>().0;

    let clock = RwSignal::new(current_time(true));
    set_interval(
        move || {
            let show_seconds =
                kernel::get_setting_bool(Owner::Os(os_category::ABOUT), about_settings::CLOCK_SECONDS_KEY, true);
            clock.set(current_time(show_seconds));
        },
        std::time::Duration::from_secs(1),
    );

    view! {
        <div class="menu-bar">
            <button
                class="menu-bar__brand"
                class:is-open=move || start_open.get()
                on:click=move |ev| {
                    ev.stop_propagation();
                    start_open.update(|o| *o = !*o);
                }
            >
                <span class="menu-bar__brand-dot"></span>
                "Aquos"
            </button>

            <Show when=move || start_open.get()>
                <div class="start-menu">
                    <div class="start-menu__label">"Applications"</div>
                    <For
                        each=move || {
                            let installed = apps.installed().all().get();
                            crate::app::ALL
                                .iter()
                                .copied()
                                .filter(move |k| installed.contains(k) && k.role() != Some(AppRole::Manipulator))
                                .collect::<Vec<_>>()
                        }
                        key=|k| *k
                        let(kind)
                    >
                        <button
                            class="start-menu__item"
                            on:click=move |_| {
                                apps.launch(kind);
                                start_open.set(false);
                            }
                        >
                            <span class="start-menu__item-icon">{kind.icon()}</span>
                            <span>{kind.title()}</span>
                        </button>
                    </For>
                </div>
            </Show>

            <div class="menu-bar__divider"></div>

            <div class="menu-bar__workspaces">
                <For each=move || workspaces.all().get() key=|w| w.id let(ws)>
                    {
                        let id = ws.id;
                        let is_active = move || workspaces.active_id().get() == id;
                        view! {
                            <button
                                class="ws-pill"
                                class:is-active=is_active
                                title=format!("Workspace {}", ws.name)
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    workspaces.set_active(id);
                                }
                            >
                                {ws.name}
                            </button>
                        }
                    }
                </For>
            </div>

            <div class="menu-bar__divider"></div>

            <div class="menu-bar__layouts">
                {LayoutKind::ALL
                    .iter()
                    .map(|kind| {
                        let kind = *kind;
                        let is_active = move || workspaces.active_workspace().layout.get() == kind;
                        view! {
                            <button
                                class="layout-pill"
                                class:is-active=is_active
                                title=kind.label()
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    workspaces.active_workspace().layout.set(kind);
                                }
                            >
                                {kind.icon()}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>

            <div class="menu-bar__divider"></div>

            <div class="menu-bar__tasks">
                <For
                    each=move || {
                        let active = workspaces.active_id().get();
                        apps.instances().get().into_iter().filter(move |i| i.workspace_id == active).collect::<Vec<_>>()
                    }
                    key=|i| i.window_id
                    let(instance)
                >
                    {
                        let window_id = instance.window_id;
                        let is_focused = move || {
                            workspaces.active_workspace().focused() == Some(window_id)
                        };
                        let is_minimized = move || {
                            workspaces.active_workspace().active_manager().is_minimized(window_id)
                        };
                        view! {
                            <button
                                class="taskbar-item"
                                class:is-focused=is_focused
                                class:is-minimized=is_minimized
                                on:click=move |_| {
                                    let workspace = workspaces.active_workspace();
                                    let manager = workspace.active_manager();
                                    if manager.is_minimized(window_id) {
                                        manager.set_minimized(window_id, false);
                                    }
                                    workspace.focus(window_id);
                                }
                            >
                                <span>{instance.kind.icon()}</span>
                                <span>{instance.kind.title()}</span>
                            </button>
                        }
                    }
                </For>
            </div>

            <button
                class="menu-bar__fullscreen"
                class:is-active=move || is_fullscreen.get()
                title=move || if is_fullscreen.get() { "Exit fullscreen" } else { "Enter fullscreen" }
                on:click=move |ev| {
                    ev.stop_propagation();
                    fullscreen::toggle();
                }
            >
                "\u{26F6}"
            </button>

            <div class="menu-bar__clock">{move || clock.get()}</div>
        </div>
    }
}

fn current_time(show_seconds: bool) -> String {
    let now = js_sys::Date::new_0();
    if show_seconds {
        format!(
            "{:02}:{:02}:{:02}",
            now.get_hours(),
            now.get_minutes(),
            now.get_seconds()
        )
    } else {
        format!("{:02}:{:02}", now.get_hours(), now.get_minutes())
    }
}
