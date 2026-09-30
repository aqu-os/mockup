//! App Store: browse the app catalog, install apps that aren't yet on the
//! system, and uninstall ones that are. Uninstalling a non-system app asks
//! whether to also wipe its personalized data (settings, and — for the
//! apps that have it — saved data like Contacts' address book).

use leptos::prelude::*;

use super::pipeline;
use super::{contacts_source, App, AppRef, Apps};
use crate::os::kernel::{self as kernel, Pid, SizeConstraints};

pub struct AppStoreApp;

impl App for AppStoreApp {
    fn id(&self) -> &'static str {
        "app-store"
    }
    fn title(&self) -> &'static str {
        "App Store"
    }
    fn description(&self) -> &'static str {
        "Install and remove apps."
    }
    fn icon(&self) -> &'static str {
        "\u{1F6CD}\u{FE0F}"
    }
    fn is_system(&self) -> bool {
        true
    }
    fn is_singleton(&self) -> bool {
        true
    }
    fn preferred_size(&self) -> (f64, f64) {
        (560.0, 460.0)
    }
    fn size_constraints(&self) -> SizeConstraints {
        SizeConstraints {
            min_width: 420.0,
            min_height: 360.0,
            max_width: None,
            max_height: None,
            aspect_ratio: None,
        }
    }
    fn render(&self, _pid: Pid) -> AnyView {
        view! { <AppStore /> }.into_any()
    }
}

#[component]
fn AppStore() -> impl IntoView {
    let apps = expect_context::<Apps>();
    let installed = apps.installed();

    let confirming: RwSignal<Option<AppRef>> = RwSignal::new(None);
    let remove_data = RwSignal::new(false);

    let confirm_uninstall = move |kind: AppRef| {
        let remove = remove_data.get_untracked();
        apps.uninstall(kind, remove);
        // Settings are cleared generically by `Apps::uninstall`, and that
        // also closes any *windowed* instance of `kind`. A source's owned
        // data block isn't a setting, and a source can also be running
        // headless (no window, so `Apps::uninstall` alone doesn't see it)
        // as part of a pipeline — tearing those down is the App Store's
        // job here.
        if remove && kind.id() == contacts_source::ID {
            for process in kernel::list_processes().into_iter().filter(|p| p.app_id == contacts_source::ID) {
                for pipeline in pipeline::pipelines_for_source(process.pid) {
                    pipeline::delete_pipeline(pipeline.id);
                }
                kernel::kill_process(process.pid);
            }
        }
        confirming.set(None);
    };

    view! {
        <div class="store">
            <div class="store__header">
                {move || {
                    let count = installed.all().get().len();
                    let total = super::ALL.len();
                    format!("{count} of {total} apps installed")
                }}
            </div>
            <div class="store__list">
                <For each=move || super::ALL.to_vec() key=|k| *k let(kind)>
                    {
                        let is_installed = move || installed.is_installed(kind);
                        let is_confirming = move || confirming.get() == Some(kind);
                        view! {
                            <div class="store-item">
                                <div class="store-item__row">
                                    <span class="store-item__icon">{kind.icon()}</span>
                                    <div class="store-item__text">
                                        <div class="store-item__title">{kind.title()}</div>
                                        <div class="store-item__desc">{kind.description()}</div>
                                    </div>
                                    <div class="store-item__action">
                                        {move || {
                                            if kind.is_system() {
                                                view! { <span class="store-item__badge">"System"</span> }
                                                    .into_any()
                                            } else if is_installed() {
                                                view! {
                                                    <button
                                                        class="store-item__btn store-item__btn--danger"
                                                        on:click=move |_| {
                                                            remove_data.set(false);
                                                            confirming.set(Some(kind));
                                                        }
                                                    >
                                                        "Uninstall"
                                                    </button>
                                                }
                                                    .into_any()
                                            } else {
                                                view! {
                                                    <button
                                                        class="store-item__btn store-item__btn--primary"
                                                        on:click=move |_| apps.install(kind)
                                                    >
                                                        "Install"
                                                    </button>
                                                }
                                                    .into_any()
                                            }
                                        }}
                                    </div>
                                </div>

                                <Show when=is_confirming>
                                    <div class="store-confirm">
                                        <p class="store-confirm__text">
                                            {format!("Uninstall {}?", kind.title())}
                                        </p>
                                        <label class="store-confirm__checkbox">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || remove_data.get()
                                                on:change=move |ev| remove_data.set(event_target_checked(&ev))
                                            />
                                            "Also remove personalized data (settings, saved data)"
                                        </label>
                                        <div class="store-confirm__actions">
                                            <button
                                                class="store-item__btn"
                                                on:click=move |_| confirming.set(None)
                                            >
                                                "Cancel"
                                            </button>
                                            <button
                                                class="store-item__btn store-item__btn--danger"
                                                on:click=move |_| confirm_uninstall(kind)
                                            >
                                                "Uninstall"
                                            </button>
                                        </div>
                                    </div>
                                </Show>
                            </div>
                        }
                    }
                </For>
            </div>
        </div>
    }
}
