//! The Program Manager app: lists every currently running program, system
//! wide (not just the current workspace), and lets the user end them.

use leptos::prelude::*;

use super::{App, Apps};
use crate::os::kernel::{self as kernel, Pid, SizeConstraints};

pub struct ProgramManagerApp;

impl App for ProgramManagerApp {
    fn id(&self) -> &'static str {
        "program-manager"
    }
    fn title(&self) -> &'static str {
        "Program Manager"
    }
    fn description(&self) -> &'static str {
        "View and manage running programs."
    }
    fn icon(&self) -> &'static str {
        "\u{1F5C2}\u{FE0F}"
    }
    fn is_system(&self) -> bool {
        true
    }
    fn is_singleton(&self) -> bool {
        true
    }
    fn preferred_size(&self) -> (f64, f64) {
        (420.0, 320.0)
    }
    fn size_constraints(&self) -> SizeConstraints {
        SizeConstraints {
            min_width: 320.0,
            min_height: 220.0,
            max_width: None,
            max_height: None,
            aspect_ratio: None,
        }
    }
    fn render(&self, _pid: Pid) -> AnyView {
        view! { <ProgramManager /> }.into_any()
    }
}

#[component]
fn ProgramManager() -> impl IntoView {
    let apps = expect_context::<Apps>();

    view! {
        <div class="pm">
            <Show
                when=move || !kernel::list_processes().is_empty()
                fallback=|| view! { <div class="pm__empty">"No programs are running."</div> }
            >
                <table class="pm__table">
                    <thead>
                        <tr>
                            <th>"Name"</th>
                            <th>"PID"</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        <For each=move || kernel::list_processes() key=|p| p.pid let(proc)>
                            {
                                let pid = proc.pid;
                                view! {
                                    <tr>
                                        <td>
                                            <span class="pm__row-name">
                                                <span>{proc.icon}</span>
                                                <span>{proc.name.clone()}</span>
                                            </span>
                                        </td>
                                        <td class="pm__pid">{pid}</td>
                                        <td>
                                            <button
                                                class="pm__end-btn"
                                                on:click=move |_| apps.close_by_pid(pid)
                                            >
                                                "End Task"
                                            </button>
                                        </td>
                                    </tr>
                                }
                            }
                        </For>
                    </tbody>
                </table>
            </Show>
        </div>
    }
}
