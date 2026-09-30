//! File Explorer: browses the real file tree the kernel hands back via
//! [`crate::os::kernel::read_root_dir`] (baked in from `./fs` at compile
//! time — see `os::kernel::fs`), plus a synthetic, always-up-to-date
//! "Applications" folder listing every installed non-system app as an
//! executable file — clicking one launches it, same as the Start Menu.
//! The kernel's file tree itself stays app-agnostic; this is the one
//! place that bridges "a file" to "a running program".

use leptos::prelude::*;

use super::{App, Apps};
use crate::os::kernel::{
    self as kernel, AppRole, DirNode, EntryMeta, FieldKind, FileKind, FileNode, Owner, Pid, SettingField,
    SettingValue, SizeConstraints,
};

/// Executable files are synthetic (an installed app, not a real file), so
/// they share one made-up-but-consistent placeholder buffer as their
/// "content" rather than tracking a real size per app.
const EXECUTABLE_SIZE_KB: usize = 128;
static EXECUTABLE_PLACEHOLDER: [u8; EXECUTABLE_SIZE_KB * 1024] = [0; EXECUTABLE_SIZE_KB * 1024];

const ID: &str = "file-explorer";
const SHOW_SIZES_KEY: &str = "show_sizes";

const SETTINGS: &[SettingField] = &[SettingField {
    key: SHOW_SIZES_KEY,
    label: "Show file sizes",
    kind: FieldKind::Toggle,
    default: SettingValue::Bool(true),
}];

pub struct FileExplorerApp;

impl App for FileExplorerApp {
    fn id(&self) -> &'static str {
        ID
    }
    fn title(&self) -> &'static str {
        "File Explorer"
    }
    fn description(&self) -> &'static str {
        "Browse the files on this system."
    }
    fn icon(&self) -> &'static str {
        "\u{1F4C1}"
    }
    fn preferred_size(&self) -> (f64, f64) {
        (560.0, 420.0)
    }
    fn size_constraints(&self) -> SizeConstraints {
        SizeConstraints {
            min_width: 420.0,
            min_height: 300.0,
            max_width: None,
            max_height: None,
            aspect_ratio: None,
        }
    }
    fn settings_manifest(&self) -> &'static [SettingField] {
        SETTINGS
    }
    fn render(&self, _pid: Pid) -> AnyView {
        view! { <FileExplorer /> }.into_any()
    }
}

#[component]
fn FileExplorer() -> impl IntoView {
    let apps = expect_context::<Apps>();
    let installed = apps.installed();
    let path: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
    let selected: RwSignal<Option<String>> = RwSignal::new(None);

    // The real ./fs tree plus a live "Applications" folder — reactive to
    // installs/uninstalls, so a File Explorer window left open sees an
    // app's executable appear or disappear the moment the App Store
    // changes it, no reopen required.
    let tree = Memo::new(move |_| {
        let mut root = kernel::read_root_dir();
        let installed_set = installed.all().get();
        let apps_dir = DirNode {
            name: "Applications",
            meta: None,
            dirs: Vec::new(),
            files: super::ALL
                .iter()
                .filter(|a| installed_set.contains(a) && !a.is_system() && a.role() != Some(AppRole::Manipulator))
                .map(|a| FileNode {
                    name: a.title(),
                    kind: FileKind::Executable,
                    meta: None,
                    content: &EXECUTABLE_PLACEHOLDER,
                })
                .collect(),
        };
        root.dirs.insert(0, apps_dir);
        root
    });

    let current_dir = Memo::new(move |_| {
        let root = tree.get();
        resolve(&root, &path.get()).cloned()
    });

    view! {
        <div class="fe">
            <div class="fe__breadcrumbs">
                <button class="fe__crumb" on:click=move |_| path.set(Vec::new())>
                    "\u{1F5A5}\u{FE0F} This PC"
                </button>
                {move || {
                    path.get()
                        .into_iter()
                        .enumerate()
                        .map(|(i, segment)| {
                            let depth = i + 1;
                            view! {
                                <span class="fe__crumb-sep">"/"</span>
                                <button
                                    class="fe__crumb"
                                    on:click=move |_| path.update(|p| p.truncate(depth))
                                >
                                    {segment}
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>

            <div class="fe__body">
                {move || {
                    let dir: DirNode = current_dir.get().unwrap_or_else(|| tree.get());
                    let show_sizes = kernel::get_setting_bool(Owner::App(ID), SHOW_SIZES_KEY, true);
                    if dir.entry_count() == 0 {
                        view! { <div class="fe__empty">"This folder is empty."</div> }.into_any()
                    } else {
                        view! {
                            <div class="fe__grid">
                                {dir
                                    .dirs
                                    .iter()
                                    .map(|d| {
                                        let name = d.name;
                                        view! {
                                            <button
                                                class="fe__entry"
                                                on:click=move |_| path.update(|p| p.push(name.to_string()))
                                            >
                                                <span class="fe__entry-icon">"\u{1F4C1}"</span>
                                                <span class="fe__entry-name">{name}</span>
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                                {dir
                                    .files
                                    .iter()
                                    .map(|f| {
                                        let name = f.name;
                                        let kind = f.kind;
                                        let meta: Option<EntryMeta> = f.meta;
                                        let is_selected = move || selected.get().as_deref() == Some(name);
                                        let on_click = move |_| {
                                            selected.set(Some(name.to_string()));
                                            if kind == FileKind::Executable {
                                                if let Some(app) = super::ALL.iter().find(|a| a.title() == name) {
                                                    apps.launch(*app);
                                                }
                                            }
                                        };
                                        let title = meta
                                            .map(|m| format!("Created {}\nModified {}", m.created, m.modified))
                                            .unwrap_or_default();
                                        view! {
                                            <button
                                                class="fe__entry fe__entry--file"
                                                class:is-selected=is_selected
                                                on:click=on_click
                                                title=title
                                            >
                                                <span class="fe__entry-icon">{f.kind.icon()}</span>
                                                <span class="fe__entry-name">{name}</span>
                                                {show_sizes
                                                    .then(|| {
                                                        view! {
                                                            <span class="fe__entry-size">
                                                                {format_size(f.content.len())}
                                                            </span>
                                                        }
                                                    })}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                            .into_any()
                    }
                }}
            </div>

            <div class="fe__statusbar">
                {move || {
                    let dir = current_dir.get();
                    let count = dir.as_ref().map(|d| d.entry_count()).unwrap_or(0);
                    let base = format!("{count} item{}", if count == 1 { "" } else { "s" });
                    let meta_info = selected
                        .get()
                        .and_then(|name| dir.as_ref()?.files.iter().find(|f| f.name == name)?.meta)
                        .map(|m| format!(" \u{2013} Created {} \u{2013} Modified {}", m.created, m.modified))
                        .unwrap_or_default();
                    format!("{base}{meta_info}")
                }}
            </div>
        </div>
    }
}

/// Walks a path of directory names from the root, returning the directory
/// at that path if every segment resolves. Pure tree-walking over data
/// already fetched from the kernel, so it lives here rather than being a
/// kernel operation itself.
fn resolve<'a>(root: &'a DirNode, path: &[String]) -> Option<&'a DirNode> {
    let mut current = root;
    for segment in path {
        current = current.dirs.iter().find(|d| d.name == segment)?;
    }
    Some(current)
}

fn format_size(size_bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let bytes = size_bytes as f64;
    if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{size_bytes} B")
    }
}
