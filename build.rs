//! Walks `./fs` at compile time and generates the code that builds the
//! real `DirNode` tree served by `src/os/kernel/fs.rs`. Every regular file
//! becomes an `include_bytes!` call (so its content is baked straight into
//! the wasm binary — no runtime fetch, no server needed to browse it), and
//! every file/folder must have a sibling `<name>.meta` file supplying its
//! `created`/`modified` timestamps or the build fails with a clear error.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let fs_root = Path::new(&manifest_dir).join("fs");
    let out_dir = env::var("OUT_DIR").unwrap();

    println!("cargo:rerun-if-changed=fs");
    let tree_expr = emit_dir(&fs_root, "Root", "None".to_string());
    let code = format!("fn build_tree() -> DirNode {{\n    {tree_expr}\n}}\n");

    fs::write(Path::new(&out_dir).join("fs_data.rs"), code).expect("failed to write generated fs tree");
}

/// Reads `dir`'s entries and emits a `dir(...)` expression for it. `name` is
/// the label the node should carry in the UI (equal to the folder name for
/// every real directory; `"Root"` for the synthetic top-level wrapper around
/// `./fs` itself, which has no sibling `.meta` of its own).
fn emit_dir(dir: &Path, name: &str, meta_expr: String) -> String {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.unwrap())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in &entries {
        let file_name = entry.file_name();
        let file_name = file_name.to_str().expect("non UTF-8 file name under fs/");

        // Sidecar `.meta` files and dotfiles (like `Downloads/.keep`, kept
        // only so git tracks the otherwise-empty folder) aren't real
        // entries — they never show up as their own node in the tree.
        if file_name.starts_with('.') || file_name.ends_with(".meta") {
            continue;
        }

        let entry_path = entry.path();
        println!("cargo:rerun-if-changed={}", entry_path.display());
        let meta_path = dir.join(format!("{file_name}.meta"));
        println!("cargo:rerun-if-changed={}", meta_path.display());
        let meta_expr = read_meta(&meta_path, file_name);

        if entry_path.is_dir() {
            dirs.push(emit_dir(&entry_path, file_name, meta_expr));
        } else {
            let kind = classify(file_name);
            let abs = entry_path
                .canonicalize()
                .unwrap_or_else(|e| panic!("cannot resolve {}: {e}", entry_path.display()));
            // Strip the `\\?\` UNC prefix Windows canonicalize() adds —
            // include_bytes! chokes on it inside a plain raw string.
            let abs = abs.display().to_string();
            let abs = abs.strip_prefix(r"\\?\").unwrap_or(&abs).to_string();
            files.push(format!(
                "file({file_name:?}, FileKind::{kind}, {meta_expr}, include_bytes!(r\"{abs}\"))"
            ));
        }
    }

    format!(
        "dir({name:?}, {meta_expr}, vec![{}], vec![{}])",
        dirs.join(", "),
        files.join(", ")
    )
}

/// Parses a sibling `.meta` file's `created:`/`modified:` fields into a
/// `Some(EntryMeta { .. })` expression. Panics (failing the build) if the
/// sidecar is missing or incomplete — every file/folder under `fs/` is
/// required to carry one.
fn read_meta(path: &Path, owner_name: &str) -> String {
    let content = fs::read_to_string(path).unwrap_or_else(|_| {
        panic!(
            "missing metadata file `{}` for `{owner_name}` — every file/folder under fs/ needs a sibling .meta file",
            path.display()
        )
    });

    let mut created = None;
    let mut modified = None;
    for line in content.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            match key.trim() {
                "created" => created = Some(value.trim().to_string()),
                "modified" => modified = Some(value.trim().to_string()),
                _ => {}
            }
        }
    }

    let created = created.unwrap_or_else(|| panic!("{}: missing `created` field", path.display()));
    let modified = modified.unwrap_or_else(|| panic!("{}: missing `modified` field", path.display()));
    format!("Some(EntryMeta {{ created: {created:?}, modified: {modified:?} }})")
}

fn classify(file_name: &str) -> &'static str {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "md" => "Text",
        "png" | "jpg" | "jpeg" | "svg" | "gif" => "Image",
        "mp3" | "wav" | "ogg" => "Audio",
        "rs" | "js" | "ts" | "py" => "Code",
        "toml" | "json" | "yaml" | "yml" => "Config",
        _ => "Binary",
    }
}
