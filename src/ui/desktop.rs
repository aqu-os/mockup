//! The desktop: wallpaper, the open-window layer for the active workspace,
//! and the menu bar.

use leptos::prelude::*;

use super::menu_bar::MenuBar;
use super::window_frame::WindowFrame;
use super::StartMenuOpen;
use crate::app::settings_manager::desktop_settings;
use crate::app::Apps;
use crate::os::kernel::{self as kernel, os_category, Owner};
use crate::os::window_manager::WindowManager as _;
use crate::os::workspace::{LayoutKind, WorkspaceManager};

#[component]
pub fn Desktop() -> impl IntoView {
    let apps = expect_context::<Apps>();
    let workspaces = expect_context::<WorkspaceManager>();
    let start_open = expect_context::<StartMenuOpen>().0;

    let wallpaper_class = move || {
        match kernel::get_setting_choice(Owner::Os(os_category::DESKTOP), desktop_settings::WALLPAPER_KEY, 0) {
            1 => "desktop--warm",
            2 => "desktop--solid",
            _ => "",
        }
    };

    let active_id = workspaces.active_id();

    // Sorted by the active manager's display order, not just insertion
    // order: tiling and infinite-scroll lay out windows via flexbox, where
    // visual order follows DOM order, so "move left"/"move right" have to
    // be reflected here for the reorder to actually be visible.
    let visible_instances = Memo::new(move |_| {
        let id = active_id.get();
        let mut instances: Vec<_> = apps
            .instances()
            .get()
            .into_iter()
            .filter(|i| i.workspace_id == id)
            .collect();
        let order = workspaces.active_workspace().active_manager().display_order();
        instances.sort_by_key(|inst| {
            order.iter().position(|&wid| wid == inst.window_id).unwrap_or(usize::MAX)
        });
        instances
    });

    let layout_class = move || {
        let modifier = match workspaces.active_workspace().layout.get() {
            LayoutKind::Floating => "desktop__windows--floating",
            LayoutKind::Tiling => "desktop__windows--tiling",
            LayoutKind::InfiniteScroll => "desktop__windows--infinite",
        };
        format!("desktop__windows {modifier}")
    };

    view! {
        <div
            class=move || format!("desktop {}", wallpaper_class())
            on:click=move |_| start_open.set(false)
        >
            <div class=layout_class>
                <For each=move || visible_instances.get() key=|i| i.window_id let(instance)>
                    <WindowFrame
                        workspace=workspaces.active_workspace()
                        id=instance.window_id
                        kind=instance.kind
                    />
                </For>
            </div>
            <MenuBar />
        </div>
    }
}
