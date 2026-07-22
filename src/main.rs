//! Desktop viewer for toMarkdownMCP: file tree + rendered Markdown pane.
//! All conversion/vault logic comes from the `to_markdown_mcp` library; this
//! crate only adds Tauri commands and Markdown→HTML rendering for the webview.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::collections::HashMap;

use notify::Watcher;
use serde::Serialize;
use tauri::Emitter;
use tauri_plugin_dialog::DialogExt;

use to_markdown_mcp::obsidian::{tools as vault_tools, vault as obsidian_vault};
use to_markdown_mcp::pipeline::convert_any_to_markdown;

mod clipboard_ops;
mod config;
mod export;
mod file_types;
mod image_zoom;
mod logging;
mod render;
mod state;
mod vault;
mod viewers;
mod word_graph;
mod commands;
use render::{render_note, RenderOpts};
use file_types::{detect_file_type, FileType};
use viewers::{CodeViewer, ImageViewer, HexViewer, FileViewer};
use state::{VaultViewerState, UserPreferences, TabMode, ZoomBehavior};
use image_zoom::ZoomCalculator;

#[derive(Serialize)]
struct TreeNode {
    name: String,
    path: String,
    is_dir: bool,
    children: Vec<TreeNode>,
}

const EXCLUDED_DIRS: &[&str] = &[
    "node_modules", "target", ".git", "__pycache__", ".venv", "dist", "build",
    ".obsidian", ".tomarkdown",
];

fn build_tree(dir: &Path, depth: usize) -> Vec<TreeNode> {
    if depth > 12 {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut nodes: Vec<TreeNode> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || EXCLUDED_DIRS.contains(&name.as_str()) {
                return None;
            }
            let is_dir = path.is_dir();
            Some(TreeNode {
                children: if is_dir { build_tree(&path, depth + 1) } else { Vec::new() },
                name,
                path: path.to_string_lossy().into_owned(),
                is_dir,
            })
        })
        .collect();
    // Directories first, then case-insensitive by name — like a file manager.
    nodes.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    nodes
}

#[tauri::command]
async fn list_tree(root: String) -> Result<Vec<TreeNode>, String> {
    let path = PathBuf::from(&root);
    if !path.is_dir() {
        return Err(format!("Not a directory: {}", root));
    }
    Ok(build_tree(&path, 0))
}

#[derive(Serialize)]
struct Rendered {
    title: String,
    html: String,
    words: usize,
    chars: usize,
    read_minutes: usize,
}


/// Polymorphic open_file command that routes to appropriate viewer based on file type.
/// Returns Rendered struct for frontend compatibility.
#[tauri::command]
async fn open_file(path: String, vault_root: Option<String>) -> Result<Rendered, String> {
    let p = PathBuf::from(&path);
    if !p.is_file() {
        return Err(format!("Not a file: {}", path));
    }

    let file_type = detect_file_type(&p);

    match file_type {
        FileType::Markdown => {
            // Handle Markdown files using existing render_note pipeline
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            let opts = RenderOpts {
                file_dir: p.parent(),
                vault_root: vault_root.as_deref().map(Path::new),
            };

            // Obsidian canvas files render via the JsonCanvas→Markdown converter
            let md = if ext == "canvas" {
                let value = vault_tools::convert_canvas(&p).map_err(|e| e.to_string())?;
                value["markdown"].as_str().unwrap_or_default().to_string()
            } else {
                std::fs::read_to_string(&p).map_err(|e| e.to_string())?
            };

            let html = render_note(&md, &opts, 0);
            let word_count = md.split_whitespace().count();
            let char_count = md.len();
            let read_minutes = (word_count + 199) / 200; // Rough: ~200 words per minute

            Ok(Rendered {
                title: p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Untitled")
                    .to_string(),
                html,
                words: word_count,
                chars: char_count,
                read_minutes,
            })
        }

        FileType::Code { language } => {
            // Handle code files with syntax highlighting
            let content = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
            let viewer = CodeViewer::new(p.clone(), language.clone(), content.clone(), false)
                .map_err(|e| e.to_string())?;
            let html = viewer.render().map_err(|e| e.to_string())?;

            let word_count = content.split_whitespace().count();
            let char_count = content.len();
            let read_minutes = (word_count + 199) / 200;

            Ok(Rendered {
                title: p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Untitled")
                    .to_string(),
                html,
                words: word_count,
                chars: char_count,
                read_minutes,
            })
        }

        FileType::Image { format } => {
            // Handle image files with dimensions
            let (width, height) = extract_image_dimensions(&p).unwrap_or((0, 0));
            let viewer = ImageViewer::new(p.clone(), format, width, height)
                .map_err(|e| e.to_string())?;
            let html = viewer.render().map_err(|e| e.to_string())?;

            Ok(Rendered {
                title: p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Image")
                    .to_string(),
                html,
                words: 0,
                chars: 0,
                read_minutes: 0,
            })
        }

        FileType::Hex => {
            // Handle binary/hex files
            let bytes = std::fs::read(&p).map_err(|e| e.to_string())?;
            let byte_count = bytes.len() as u64;
            let viewer = HexViewer::new_from_bytes(p.clone(), bytes, byte_count)
                .map_err(|e| e.to_string())?;
            let html = viewer.render().map_err(|e| e.to_string())?;

            Ok(Rendered {
                title: p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Binary")
                    .to_string(),
                html,
                words: 0,
                chars: 0,
                read_minutes: 0,
            })
        }
    }
}

/// Extract image dimensions from a file using the image crate.
/// Returns (width, height) on success, None if dimensions cannot be determined.
fn extract_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    use image::ImageReader;
    let reader = ImageReader::open(path).ok()?;
    let dimensions = reader.into_dimensions().ok()?;
    Some(dimensions)
}

/// View binary file in specified format
/// Supports: "hex" (hex dump), "text" (raw UTF-8), "base64"
#[tauri::command]
async fn view_binary(path: String, format: String) -> Result<Rendered, String> {
    let p = PathBuf::from(&path);
    if !p.is_file() {
        return Err(format!("Not a file: {}", path));
    }

    let bytes = std::fs::read(&p).map_err(|e| e.to_string())?;
    let byte_count = bytes.len() as u64;
    let viewer = HexViewer::new_from_bytes(p.clone(), bytes, byte_count)
        .map_err(|e| e.to_string())?;

    let html = match format.as_str() {
        "hex" => viewer.render_hex_html(),
        "text" => viewer.render_text_html(),
        "base64" => viewer.render_base64_html(),
        _ => viewer.render_hex_html(), // default to hex
    };

    Ok(Rendered {
        title: p.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Binary")
            .to_string(),
        html,
        words: 0,
        chars: 0,
        read_minutes: 0,
    })
}

/// Live watchers: one for the open folder tree, one for the parent of the
/// open file (editors replace files on save, so the file itself would lose
/// its inode). Setting a new watcher drops and replaces the previous one.
#[derive(Default)]
struct WatchState {
    tree: Mutex<Option<notify::RecommendedWatcher>>,
    file: Mutex<Option<notify::RecommendedWatcher>>,
}

fn make_watcher(
    app: tauri::AppHandle,
    event_name: &'static str,
    target: &Path,
    mode: notify::RecursiveMode,
) -> Result<notify::RecommendedWatcher, String> {
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            // Only content-affecting events; skip pure access notifications.
            if matches!(
                event.kind,
                notify::EventKind::Create(_) | notify::EventKind::Modify(_) | notify::EventKind::Remove(_)
            ) {
                let paths: Vec<String> =
                    event.paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                let _ = app.emit(event_name, paths);
            }
        }
    })
    .map_err(|e| e.to_string())?;
    watcher.watch(target, mode).map_err(|e| e.to_string())?;
    Ok(watcher)
}

#[tauri::command]
fn watch_tree(
    app: tauri::AppHandle,
    state: tauri::State<WatchState>,
    root: String,
) -> Result<(), String> {
    let watcher = make_watcher(app, "tree-changed", Path::new(&root), notify::RecursiveMode::Recursive)?;
    *state.tree.lock().unwrap() = Some(watcher);
    Ok(())
}

#[tauri::command]
fn watch_file(
    app: tauri::AppHandle,
    state: tauri::State<WatchState>,
    path: String,
) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let parent = p.parent().unwrap_or(Path::new(".")).to_path_buf();
    let watcher = make_watcher(app, "file-changed", &parent, notify::RecursiveMode::NonRecursive)?;
    *state.file.lock().unwrap() = Some(watcher);
    Ok(())
}

/// Save a standalone styled HTML export of the current document.
#[tauri::command]
async fn export_html(
    app: tauri::AppHandle,
    title: String,
    body_html: String,
    css: String,
) -> Result<Option<String>, String> {
    let Some(dest) = app
        .dialog()
        .file()
        .set_file_name(format!("{}.html", title.trim_end_matches(".md")))
        .add_filter("HTML", &["html"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = dest.into_path().map_err(|e| e.to_string())?;
    let doc = format!(
        "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n<style>\n{}\n</style>\n</head>\n<body>\n<main class=\"content\">\n{}\n</main>\n</body>\n</html>\n",
        title, css, body_html
    );
    std::fs::write(&path, doc).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

/// Vault-relative path of `abs` under `root`, if it is inside the vault.
fn vault_rel(root: &str, abs: &str) -> Option<String> {
    let root = PathBuf::from(root).canonicalize().ok()?;
    let abs = PathBuf::from(abs).canonicalize().ok()?;
    abs.strip_prefix(&root).ok().map(|p| p.to_string_lossy().into_owned())
}

/// Everything the note panel needs for the open file: properties
/// (frontmatter), tags, headings, outgoing links, backlinks, Dataview fields.
#[tauri::command]
fn note_info(root: String, path: String) -> Result<serde_json::Value, String> {
    let rel = vault_rel(&root, &path).ok_or("File is outside the vault")?;
    let rootp = Path::new(&root);
    let note = vault_tools::get_note(rootp, &rel, false, 0).map_err(|e| e.to_string())?;
    let backlinks = vault_tools::get_backlinks(rootp, &rel).map_err(|e| e.to_string())?;
    let fields = vault_tools::extract_dataview_fields(rootp, Some(&rel), None).unwrap_or_default();
    Ok(serde_json::json!({ "note": note, "backlinks": backlinks, "fields": fields }))
}

#[tauri::command]
async fn vault_overview(root: String) -> Result<serde_json::Value, String> {
    vault_tools::vault_index(Path::new(&root), true).map_err(|e| e.to_string())
}

#[tauri::command]
async fn vault_search(root: String, query: String, mode: String) -> Result<serde_json::Value, String> {
    vault_tools::search(Path::new(&root), &query, &mode, 50).map_err(|e| e.to_string())
}

#[tauri::command]
async fn vault_tasks(root: String) -> Result<serde_json::Value, String> {
    vault_tools::list_tasks(Path::new(&root), None, None).map_err(|e| e.to_string())
}

/// Resolve a wikilink target to an absolute path, using Obsidian's
/// shortest-path rules relative to the note the click came from.
#[tauri::command]
fn resolve_wikilink(root: String, target: String, from: String) -> Result<String, String> {
    let idx = obsidian_vault::get_index(Path::new(&root)).map_err(|e| e.to_string())?;
    let from_rel = vault_rel(&root, &from);
    match obsidian_vault::resolve_target(&idx, &target, from_rel.as_deref()) {
        obsidian_vault::Resolution::Resolved(rel) => Ok(Path::new(&root).join(rel).to_string_lossy().into_owned()),
        obsidian_vault::Resolution::Ambiguous(mut candidates) => {
            // Obsidian opens the first shortest-path candidate.
            candidates.sort_by_key(|c| c.len());
            candidates
                .first()
                .map(|rel| Path::new(&root).join(rel).to_string_lossy().into_owned())
                .ok_or_else(|| format!("Ambiguous link with no candidates: {}", target))
        }
        obsidian_vault::Resolution::Broken => Err(format!("Broken link: {}", target)),
    }
}

/// Nodes and links for the vault graph view. `focus` narrows to the direct
/// neighborhood of one note (local graph).
#[tauri::command]
async fn graph_data(root: String, focus: Option<String>) -> Result<serde_json::Value, String> {
    let idx = obsidian_vault::get_index(Path::new(&root)).map_err(|e| e.to_string())?;
    let mut edges: Vec<(String, String)> = Vec::new();
    for (from, links) in &idx.links {
        for link in links {
            if let obsidian_vault::Resolution::Resolved(to) = obsidian_vault::resolve_target(&idx, &link.target, Some(from)) {
                edges.push((from.clone(), to));
            }
        }
    }
    let focus_rel = focus.as_deref().and_then(|f| vault_rel(&root, f));
    if let Some(center) = &focus_rel {
        edges.retain(|(a, b)| a == center || b == center);
    }
    let mut node_set: std::collections::BTreeSet<String> = edges
        .iter()
        .flat_map(|(a, b)| [a.clone(), b.clone()])
        .collect();
    match &focus_rel {
        Some(center) => {
            node_set.insert(center.clone());
        }
        // The global graph also shows unlinked notes.
        None => node_set.extend(idx.notes.keys().cloned()),
    }
    let nodes: Vec<serde_json::Value> = node_set
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p,
                "title": idx.notes.get(p).map(|n| n.title.clone()).unwrap_or_else(|| p.clone()),
                "links": edges.iter().filter(|(a, b)| a == p || b == p).count(),
            })
        })
        .collect();
    let links: Vec<serde_json::Value> = edges
        .iter()
        .map(|(a, b)| serde_json::json!({ "source": a, "target": b }))
        .collect();
    Ok(serde_json::json!({ "nodes": nodes, "links": links }))
}

/// A ranked wikilink-completion candidate.
#[derive(serde::Serialize, Debug)]
struct WikiMatch {
    label: String,
    path: String,
}

/// Rank vault notes against `prefix`: 0 = title-prefix (or empty prefix), 1 =
/// title-substring, 2 = alias-substring; case-insensitive, ties alphabetical.
fn wikilink_matches(root: &Path, prefix: &str) -> Result<Vec<WikiMatch>, String> {
    let idx = obsidian_vault::get_index(root).map_err(|e| e.to_string())?;
    let q = prefix.to_lowercase();
    let mut ranked: Vec<(u8, String, String)> = Vec::new();
    for n in idx.notes.values() {
        let t = n.title.to_lowercase();
        let rank = if q.is_empty() || t.starts_with(&q) {
            0
        } else if t.contains(&q) {
            1
        } else if n.aliases.iter().any(|a| a.to_lowercase().contains(&q)) {
            2
        } else {
            continue;
        };
        ranked.push((rank, n.title.clone(), n.path.clone()));
    }
    ranked.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    Ok(ranked
        .into_iter()
        .take(20)
        .map(|(_, label, path)| WikiMatch { label, path })
        .collect())
}

/// Top 20 vault-note completions for a wikilink query, ranked by relevance.
#[tauri::command]
fn wikilink_complete(root: String, prefix: String) -> Result<Vec<WikiMatch>, String> {
    wikilink_matches(Path::new(&root), &prefix)
}

/// Titles + aliases for the quick switcher's fuzzy matching.
#[tauri::command]
fn quick_list(root: String) -> Result<serde_json::Value, String> {
    let idx = obsidian_vault::get_index(Path::new(&root)).map_err(|e| e.to_string())?;
    let mut items: Vec<serde_json::Value> = idx
        .notes
        .values()
        .map(|n| serde_json::json!({ "path": n.path, "title": n.title, "aliases": n.aliases }))
        .collect();
    items.sort_by(|a, b| a["title"].as_str().cmp(&b["title"].as_str()));
    Ok(serde_json::json!(items))
}

// ---- Editing commands (Phase 8) ----

/// Raw source for the editor, capped at the structured-conversion limit.
#[tauri::command]
fn read_source(path: String) -> Result<String, String> {
    let p = PathBuf::from(&path);
    let size = std::fs::metadata(&p).map(|m| m.len()).map_err(|e| e.to_string())?;
    if size > to_markdown_mcp::pipeline::LARGE_FILE_BYTES {
        return Err("File too large to edit here".to_string());
    }
    std::fs::read_to_string(&p).map_err(|e| e.to_string())
}

/// Atomic save: write a temp file next to the target, then rename over it.
/// Invalidates the vault index so links/backlinks stay fresh.
#[tauri::command]
fn save_file(path: String, content: String, vault_root: Option<String>) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let tmp = p.with_extension("tomarkdown.tmp");
    std::fs::write(&tmp, &content).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())?;
    if let Some(root) = vault_root {
        obsidian_vault::invalidate(Path::new(&root));
    }
    Ok(())
}

/// Render markdown for the edit-mode live preview (same pipeline as open_file).
#[tauri::command]
fn render_markdown(md: String, vault_root: Option<String>, file_path: Option<String>) -> String {
    let file_dir = file_path.as_deref().and_then(|p| Path::new(p).parent().map(Path::to_path_buf));
    let opts = RenderOpts {
        file_dir: file_dir.as_deref(),
        vault_root: vault_root.as_deref().map(Path::new),
    };
    render_note(&md, &opts, 0)
}

/// Split a document into lossless blocks and render each — the live editor's
/// data model. Rejoining the returned texts reproduces the document exactly.
#[tauri::command]
async fn render_blocks(
    md: String,
    vault_root: Option<String>,
    file_path: Option<String>,
) -> Vec<serde_json::Value> {
    let file_dir = file_path.as_deref().and_then(|p| Path::new(p).parent().map(Path::to_path_buf));
    let opts = RenderOpts {
        file_dir: file_dir.as_deref(),
        vault_root: vault_root.as_deref().map(Path::new),
    };
    render::split_blocks(&md)
        .into_iter()
        .map(|text| {
            let html = render::render_block_cached(&text, &opts);
            serde_json::json!({ "text": text, "html": html })
        })
        .collect()
}

/// Class-based syntect CSS for both themes; injected once by the frontend.
#[tauri::command]
fn syntax_css() -> String {
    render::syntax_css()
}

/// Toggle the nth checkbox task in a file (0-indexed, document order),
/// matching how the rendered preview orders its checkboxes.
#[tauri::command]
fn toggle_task(path: String, index: usize, vault_root: Option<String>) -> Result<(), String> {
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut seen = 0usize;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let mut found = false;
    for line in lines.iter_mut() {
        let trimmed = line.trim_start();
        let is_task = (trimmed.starts_with("- [") || trimmed.starts_with("* [") || trimmed.starts_with("+ ["))
            && trimmed.as_bytes().get(4) == Some(&b']');
        if is_task {
            if seen == index {
                let state = trimmed.as_bytes()[3] as char;
                let new_state = if state == ' ' { 'x' } else { ' ' };
                let pos = line.len() - trimmed.len() + 3;
                line.replace_range(pos..pos + 1, &new_state.to_string());
                found = true;
                break;
            }
            seen += 1;
        }
    }
    if !found {
        return Err(format!("No task #{} in {}", index, path));
    }
    let mut out = lines.join("\n");
    if content.ends_with('\n') {
        out.push('\n');
    }
    save_file(path, out, vault_root)
}

/// Create a note (optionally from a template / as today's daily note) and
/// return its absolute path.
#[tauri::command]
fn create_note(root: String, title: Option<String>, daily: bool, template: Option<String>) -> Result<String, String> {
    let v = vault_tools::create_note_from_template(Path::new(&root), title.as_deref(), template.as_deref(), daily)
        .map_err(|e| e.to_string())?;
    obsidian_vault::invalidate(Path::new(&root));
    let rel = v["created"].as_str().ok_or("create returned no path")?;
    Ok(Path::new(&root).join(rel).to_string_lossy().into_owned())
}

/// Rename/move a note, rewriting every inbound wikilink (real run).
#[tauri::command]
fn rename_note(root: String, path: String, new_name: String) -> Result<String, String> {
    let rel = vault_rel(&root, &path).ok_or("File is outside the vault")?;
    let v = vault_tools::rename_note(Path::new(&root), &rel, &new_name, false)
        .map_err(|e| e.to_string())?;
    obsidian_vault::invalidate(Path::new(&root));
    let new_rel = v["renamed_to"].as_str().unwrap_or(&new_name);
    Ok(Path::new(&root).join(new_rel).to_string_lossy().into_owned())
}

/// Replace the YAML frontmatter block of a note with the given YAML text
/// (empty string removes it), leaving the body untouched.
#[tauri::command]
fn set_frontmatter(path: String, yaml: String, vault_root: Option<String>) -> Result<(), String> {
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let (_, body) = to_markdown_mcp::obsidian::frontmatter::split(&content);
    let yaml = yaml.trim();
    let new_content = if yaml.is_empty() {
        body.to_string()
    } else {
        // Validate before writing so a typo can't corrupt the note.
        serde_yaml::from_str::<serde_yaml::Value>(yaml).map_err(|e| format!("Invalid YAML: {}", e))?;
        format!("---\n{}\n---\n{}", yaml, body)
    };
    save_file(path, new_content, vault_root)
}

/// Resolve (and create) the vault's configured attachment folder.
fn attachment_dir(root: &Path) -> Result<PathBuf, String> {
    let cfg = to_markdown_mcp::obsidian::config::read_config(root).unwrap_or_default();
    let folder = cfg.attachment_folder.unwrap_or_default();
    let dir = if folder.is_empty() || folder == "/" {
        root.to_path_buf()
    } else {
        root.join(folder.trim_start_matches("./"))
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Save a pasted image into the vault's attachment folder and return the
/// wikilink embed text to insert.
#[tauri::command]
fn paste_image(root: String, base64_data: String, extension: String) -> Result<String, String> {
    let bytes = base64_decode(&base64_data).ok_or("Invalid base64 image data")?;
    let dir = attachment_dir(Path::new(&root))?;
    let name = format!("Pasted image {}.{}", chrono_stamp(), extension);
    std::fs::write(dir.join(&name), bytes).map_err(|e| e.to_string())?;
    obsidian_vault::invalidate(Path::new(&root));
    Ok(format!("![[{}]]", name))
}

#[derive(serde::Serialize, Debug)]
struct StoredAttachment {
    name: String,
    embed: String,
}

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "tiff", "tif"];

/// Find a filename of the form `stem.ext` (or `stem N.ext` for `N >= 2`) that
/// does not already exist in `dir`.
fn dedupe_in(dir: &Path, stem: &str, ext: &str) -> String {
    let mut name = if ext.is_empty() { stem.to_string() } else { format!("{}.{}", stem, ext) };
    let mut n = 2;
    while dir.join(&name).exists() {
        name = if ext.is_empty() { format!("{} {}", stem, n) } else { format!("{} {}.{}", stem, n, ext) };
        n += 1;
    }
    name
}

fn store_attachment_impl(root: &Path, src: &Path) -> Result<StoredAttachment, String> {
    if !src.is_file() {
        return Err(format!("File not found: {}", src.display()));
    }
    let dir = attachment_dir(root)?;
    let stem = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    let ext = src.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let name = dedupe_in(&dir, &stem, &ext);
    std::fs::copy(src, dir.join(&name)).map_err(|e| e.to_string())?;
    obsidian_vault::invalidate(root);
    let embed = if IMAGE_EXTS.contains(&ext.as_str()) { format!("![[{}]]", name) } else { format!("[[{}]]", name) };
    Ok(StoredAttachment { name, embed })
}

#[tauri::command]
fn store_attachment(root: String, src_path: String) -> Result<StoredAttachment, String> {
    store_attachment_impl(Path::new(&root), Path::new(&src_path))
}

/// Derive a filename for a downloaded URL, falling back to a timestamped
/// name and a content-type-derived extension when the URL path is
/// uninformative.
/// Minimal percent-decoding for URL path segments (%20 → space, etc.).
/// Invalid sequences are kept literally; only valid UTF-8 results are used.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let (Some(h), Some(l)) = (
                bytes.get(i + 1).and_then(|b| (*b as char).to_digit(16)),
                bytes.get(i + 2).and_then(|b| (*b as char).to_digit(16)),
            ) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

fn ext_for_content_type(content_type: Option<&str>) -> Option<&'static str> {
    match content_type.map(|c| c.split(';').next().unwrap_or(c).trim()) {
        Some("image/jpeg") => Some("jpg"),
        Some("image/png") => Some("png"),
        Some("image/gif") => Some("gif"),
        Some("image/webp") => Some("webp"),
        Some("image/svg+xml") => Some("svg"),
        Some("image/bmp") => Some("bmp"),
        Some("application/pdf") => Some("pdf"),
        _ => None,
    }
}

fn url_attachment_name(url: &str, content_type: Option<&str>, server_name: Option<&str>) -> String {
    // Content-Disposition filename is the authoritative original name.
    if let Some(n) = server_name {
        let n = n.trim();
        if !n.is_empty() {
            return n.to_string();
        }
    }
    let path_part = url.split('?').next().unwrap_or(url).split('#').next().unwrap_or(url);
    let last = path_part.trim_end_matches('/').rsplit('/').next().unwrap_or("");
    let last = percent_decode(last);
    let last = last.replace(['/', '\\'], "_");
    let mut name = if last.is_empty() || last.contains("://") {
        format!("Downloaded {}", chrono_stamp())
    } else {
        last
    };
    // Ensure the extension reflects the actual content: append one when the
    // name has none; the URL-derived extension is kept when present (it is
    // part of the original filename).
    if !name.contains('.') {
        name = format!("{}.{}", name, ext_for_content_type(content_type).unwrap_or("bin"));
    }
    name
}

#[tauri::command]
async fn store_url_attachment(root: String, url: String) -> Result<StoredAttachment, String> {
    let (bytes, ctype, server_name) =
        to_markdown_mcp::sources::fetch_url_bytes(&url).await.map_err(|e| e.to_string())?;
    let name0 = url_attachment_name(&url, ctype.as_deref(), server_name.as_deref());
    let rootp = Path::new(&root);
    let dir = attachment_dir(rootp)?;
    let (stem, ext) = match name0.rsplit_once('.') { Some((s, e)) => (s.to_string(), e.to_string()), None => (name0.clone(), String::new()) };
    let name = dedupe_in(&dir, &stem, &ext);
    std::fs::write(dir.join(&name), bytes).map_err(|e| e.to_string())?;
    obsidian_vault::invalidate(rootp);
    let e = name.rsplit('.').next().unwrap_or("").to_lowercase();
    let embed = if IMAGE_EXTS.contains(&e.as_str()) { format!("![[{}]]", name) } else { format!("[[{}]]", name) };
    Ok(StoredAttachment { name, embed })
}

/// An external (non-vault) link target found in a note: a URL, image URL,
/// or `file://` reference that could be localized into the vault.
#[derive(serde::Serialize, Debug)]
struct ExtLink {
    link: String,
    kind: String,
}

/// Scan `note` for `[text](target)` / `![alt](target)` constructs whose
/// target is an external `http(s)://` or `file://` URL (vault-internal
/// wikilinks and relative paths are excluded). Duplicate targets are
/// collapsed to a single entry.
fn scan_external_links_impl(_root: &Path, note: &Path) -> Result<Vec<ExtLink>, String> {
    let src = std::fs::read_to_string(note).map_err(|e| e.to_string())?;
    let mut out: Vec<ExtLink> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // Markdown links/images: ![alt](target) and [text](target)
    let mut i = 0;
    while let Some(open) = src[i..].find("](") {
        let start = i + open + 2;
        if let Some(close) = src[start..].find(')') {
            let target =
                src[start..start + close].split_whitespace().next().unwrap_or("").to_string();
            i = start + close + 1;
            // `start - 2` is the `]` that opens `](`; walk back through
            // nested brackets (e.g. `[a[b]](url)`) to find its true match.
            let is_image = find_matching_open_bracket(&src, start - 2)
                .map(|b| b > 0 && src.as_bytes()[b - 1] == b'!')
                .unwrap_or(false);
            let kind = if target.starts_with("http://") || target.starts_with("https://") {
                let clean = target.split('?').next().unwrap_or(&target);
                let ext = clean.rsplit('.').next().unwrap_or("").to_lowercase();
                if is_image || IMAGE_EXTS.contains(&ext.as_str()) { "image_url" } else { "url" }
            } else if target.starts_with("file://") {
                "file"
            } else {
                continue;
            };
            if seen.insert(target.clone()) {
                out.push(ExtLink { link: target, kind: kind.into() });
            }
        } else {
            break;
        }
    }
    Ok(out)
}

/// Convert a `file://` (or `file://localhost/...`) URL to a filesystem
/// path. Percent-decoding is intentionally out of scope: paths containing
/// `%xx` escapes are not currently supported.
fn file_url_to_path(url: &str) -> &str {
    let rest = url.trim_start_matches("file://");
    rest.strip_prefix("localhost").unwrap_or(rest)
}

/// Resolve `link` + `action` (`store` | `convert`) to the wikilink/embed
/// text that should replace it in the note.
async fn localize_target(root: &Path, link: &str, action: &str) -> Result<String, String> {
    match (action, link) {
        ("store", l) if l.starts_with("file://") => {
            let p = file_url_to_path(l);
            Ok(store_attachment_impl(root, Path::new(p))?.embed)
        }
        ("store", l) => Ok(store_url_attachment(root.display().to_string(), l.to_string()).await?.embed),
        ("convert", l) => {
            let conv = if l.starts_with("file://") {
                convert_file_to_markdown(file_url_to_path(l).to_string()).await?
            } else {
                convert_url_to_markdown(l.to_string()).await?
            };
            let path = save_markdown_to_imports(root, &conv.suggested_name, &conv.markdown)?;
            let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            Ok(format!("[[{}]]", title))
        }
        _ => Err(format!("Unknown action: {}", action)),
    }
}

/// Walk backward from `close_pos` (the byte index of a `]`) tracking
/// bracket depth to find the index of its matching `[`, correctly skipping
/// over nested brackets such as the inner `[b]` in `[a[b]](url)`.
fn find_matching_open_bracket(s: &str, close_pos: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut i = close_pos;
    loop {
        match bytes[i] {
            b']' => depth += 1,
            b'[' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        if i == 0 {
            return None;
        }
        i -= 1;
    }
}

/// Find every `[text](link ...)` / `![alt](link ...)` construct in `src`
/// whose target (the first whitespace-separated token inside the parens,
/// same extraction as `scan_external_links_impl`) equals `link` exactly —
/// so a title-bearing form like `[t](url "title")` matches, but a longer
/// URL that merely starts with `link` does not. Returns the byte range of
/// the whole construct (from `[`/`![` through the closing `)`), in
/// left-to-right order.
fn find_link_occurrences(src: &str, link: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(open) = src[i..].find("](") {
        let start = i + open + 2;
        let Some(close) = src[start..].find(')') else { break };
        let end = start + close + 1;
        let target = src[start..start + close].split_whitespace().next().unwrap_or("");
        if target == link {
            // `start - 2` is the byte index of the `]` that opens `](`.
            if let Some(bracket_open) = find_matching_open_bracket(src, start - 2) {
                let bang = bracket_open > 0 && src.as_bytes()[bracket_open - 1] == b'!';
                let range_start = if bang { bracket_open - 1 } else { bracket_open };
                out.push((range_start, end));
            }
        }
        i = end;
    }
    out
}

/// Replace every `[text](link)` / `![alt](link)` occurrence in `note`
/// (including title-bearing forms like `[text](link "title")`) with the
/// localized wikilink/embed produced by `action`, and persist the updated
/// source. Returns the full updated note text.
///
/// Verifies at least one occurrence exists *before* calling
/// `localize_target`, so a link that isn't actually present in the note
/// never triggers the store/convert side effect (network fetch, file copy,
/// or Imports write).
async fn localize_link_impl(root: &Path, note: &Path, link: &str, action: &str) -> Result<String, String> {
    let src = std::fs::read_to_string(note).map_err(|e| e.to_string())?;
    if find_link_occurrences(&src, link).is_empty() {
        return Err("link not found in note".into());
    }
    let replacement = localize_target(root, link, action).await?;
    // `replacement` is always a wikilink (`[[...]]` / `![[...]]`) and can
    // never itself contain a `](` construct, so re-scanning after each
    // replacement strictly shrinks the occurrence count and this loop
    // terminates.
    let mut updated = src;
    while let Some(&(start, end)) = find_link_occurrences(&updated, link).first() {
        updated.replace_range(start..end, &replacement);
    }
    std::fs::write(note, &updated).map_err(|e| e.to_string())?;
    obsidian_vault::invalidate(root);
    Ok(updated)
}

#[tauri::command]
fn scan_external_links(root: String, note_path: String) -> Result<Vec<ExtLink>, String> {
    scan_external_links_impl(Path::new(&root), Path::new(&note_path))
}

#[tauri::command]
async fn localize_link(root: String, note_path: String, link: String, action: String) -> Result<String, String> {
    localize_link_impl(Path::new(&root), Path::new(&note_path), &link, &action).await
}

fn chrono_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("{}", secs)
}

fn base64_decode(data: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(data.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u8;
    for &b in data.as_bytes() {
        if b == b'=' || b == b'\n' || b == b'\r' {
            continue;
        }
        let v = TABLE.iter().position(|&t| t == b)? as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

/// Tag names for the editor's `#` autocomplete.
#[tauri::command]
fn tag_list(root: String) -> Result<Vec<String>, String> {
    let idx = obsidian_vault::get_index(Path::new(&root)).map_err(|e| e.to_string())?;
    let mut tags: Vec<String> = idx.tags.keys().cloned().collect();
    tags.sort();
    Ok(tags)
}

// ---- Intelligence (Phase 9) ----

/// Notes similar to the given one, ranked by TF-cosine over the vault.
#[tauri::command]
async fn related_notes(root: String, path: String) -> Result<serde_json::Value, String> {
    use to_markdown_mcp::knowledge;
    let idx = obsidian_vault::get_index(Path::new(&root)).map_err(|e| e.to_string())?;
    let target_rel = vault_rel(&root, &path);
    let target = convert_any_to_markdown(Path::new(&path)).map_err(|e| e.to_string())?;
    let ttf = knowledge::term_frequencies(&target);
    let mut results: Vec<(String, f64)> = idx
        .notes
        .keys()
        .filter(|p| Some(p.as_str()) != target_rel.as_deref())
        .filter_map(|p| {
            let content = std::fs::read_to_string(Path::new(&root).join(p)).ok()?;
            let score = knowledge::cosine_similarity(&ttf, &knowledge::term_frequencies(&content));
            (score > 0.0).then_some((p.clone(), score))
        })
        .collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(8);
    Ok(serde_json::json!(results
        .into_iter()
        .map(|(p, s)| serde_json::json!({"path": p, "score": s}))
        .collect::<Vec<_>>()))
}

/// Vector-similarity search over the vault, using the persistent
/// `.tomarkdown` embedding index (hashed-vector fallback without a model).
#[tauri::command]
async fn semantic_search(root: String, query: String) -> Result<serde_json::Value, String> {
    use to_markdown_mcp::embeddings;
    let rootp = PathBuf::from(&root);
    let idx = obsidian_vault::get_index(&rootp).map_err(|e| e.to_string())?;
    let sources: Vec<PathBuf> = idx.notes.keys().map(|p| rootp.join(p)).collect();
    let mut embedder = embeddings::default_embedder();
    let mut vindex = embeddings::VectorIndex::load(&rootp);
    vindex
        .update(&sources, embedder.as_mut(), |p| convert_any_to_markdown(p).ok())
        .map_err(|e| e.to_string())?;
    let _ = vindex.save(&rootp);
    let qv = embedder.embed(std::slice::from_ref(&query)).map_err(|e| e.to_string())?;
    let hits: Vec<serde_json::Value> = vindex
        .rank(&qv[0])
        .into_iter()
        .filter(|(_, _, s)| *s > 0.0)
        .take(12)
        .map(|(source, chunk, score)| {
            let snippet: String = chunk.text.chars().take(160).collect();
            serde_json::json!({
                "path": source,
                "heading": chunk.heading_path.join(" › "),
                "score": score,
                "snippet": snippet,
            })
        })
        .collect();
    Ok(serde_json::json!(hits))
}

/// Store/clear the Anthropic API key for this process (enables ai actions).
#[tauri::command]
fn set_api_key(key: String) {
    if key.trim().is_empty() {
        std::env::remove_var("ANTHROPIC_API_KEY");
    } else {
        std::env::set_var("ANTHROPIC_API_KEY", key.trim());
    }
}

/// Claude-backed actions on the open document (needs the API key).
#[tauri::command]
async fn ai_action(kind: String, content: String, extra: Option<String>) -> Result<String, String> {
    use to_markdown_mcp::llm;
    if llm::api_key().is_none() {
        return Err("No Anthropic API key. Add one in Settings (⌘,) to enable AI actions.".to_string());
    }
    let doc: String = content.chars().take(40_000).collect();
    let (system, prompt, max_tokens) = match kind.as_str() {
        "summarize" => (
            "You summarize documents faithfully and concisely in Markdown.",
            format!("Summarize the following document. Lead with a one-paragraph TL;DR, then key points as bullets.\n\n---\n{}", doc),
            700,
        ),
        "tag" => (
            "You suggest topical tags for notes.",
            format!("Suggest 3-8 topical tags for this document as a single line of #kebab-case tags, nothing else.\n\n---\n{}", doc),
            120,
        ),
        "translate" => (
            "You translate documents preserving all Markdown structure.",
            format!("Translate the following document to {}. Preserve Markdown exactly.\n\n---\n{}", extra.as_deref().unwrap_or("English"), doc),
            4000,
        ),
        "ask" => (
            "You answer questions grounded strictly in the provided document. Say so when the document doesn't contain the answer.",
            format!("Document:\n---\n{}\n---\n\nQuestion: {}", doc, extra.as_deref().unwrap_or("Summarize this.")),
            900,
        ),
        other => return Err(format!("Unknown AI action: {}", other)),
    };
    let model = llm::resolve_model(None);
    llm::complete(&prompt, Some(system), &model, max_tokens)
        .await
        .map_err(|e| e.to_string())
}

// ---- Phase B: document stats & hover peek ----

/// Marked 2-style document statistics: readability + structure + top words.
#[tauri::command]
async fn doc_stats(content: String) -> serde_json::Value {
    use to_markdown_mcp::{doc_intel, rag};
    let r = doc_intel::readability(&content);
    let stats = rag::text_statistics(&content, true, 3);
    let top_words: Vec<serde_json::Value> = stats
        .frequencies
        .iter()
        .take(15)
        .map(|(w, c)| serde_json::json!({"word": w, "count": c}))
        .collect();
    serde_json::json!({
        "flesch_reading_ease": r.flesch_reading_ease,
        "flesch_kincaid_grade": r.flesch_kincaid_grade,
        "interpretation": doc_intel::flesch_interpretation(r.flesch_reading_ease),
        "words": r.words,
        "sentences": r.sentences,
        "avg_sentence_length": r.avg_sentence_length,
        "distinct_words": stats.distinct_words,
        "top_words": top_words,
    })
}

/// Rendered snippet of a wikilink target, for the hover page-preview.
/// Resolves relative to `from` (the note being read); broken links and
/// non-Markdown targets return an informational preview instead of an error
/// so every hover shows something.
#[tauri::command]
async fn peek_note(root: String, target: String, from: Option<String>) -> Result<serde_json::Value, String> {
    let rootp = PathBuf::from(&root);
    let idx = obsidian_vault::get_index(&rootp).map_err(|e| e.to_string())?;
    let from_rel = from.as_deref().and_then(|f| vault_rel(&root, f));
    let rel = match obsidian_vault::resolve_target(&idx, &target, from_rel.as_deref()) {
        obsidian_vault::Resolution::Resolved(r) => r,
        obsidian_vault::Resolution::Ambiguous(mut c) => {
            c.sort_by_key(|p| p.len());
            match c.into_iter().next() {
                Some(r) => r,
                None => return Ok(peek_missing(&target)),
            }
        }
        obsidian_vault::Resolution::Broken => return Ok(peek_missing(&target)),
    };
    let abs = rootp.join(&rel);
    let is_md = abs.extension().and_then(|e| e.to_str()).is_none_or(|e| e == "md");
    if !is_md {
        // Non-note target (canvas, attachment, ...): show basic file info.
        let size = std::fs::metadata(&abs).map(|m| m.len()).unwrap_or(0);
        return Ok(serde_json::json!({
            "path": rel,
            "html": format!(
                "<em>{} — {:.1} KB. Click the link to open it.</em>",
                abs.extension().and_then(|e| e.to_str()).unwrap_or("file"),
                size as f64 / 1024.0
            ),
            "truncated": false,
        }));
    }
    let content = std::fs::read_to_string(&abs).map_err(|e| e.to_string())?;
    let (_, body) = to_markdown_mcp::obsidian::frontmatter::split(&content);
    let snippet: String = body.lines().take(30).collect::<Vec<_>>().join("\n");
    let opts = RenderOpts { file_dir: abs.parent(), vault_root: Some(&rootp) };
    Ok(serde_json::json!({
        "path": rel,
        "html": render_note(&snippet, &opts, 1),
        "truncated": body.lines().count() > 30,
    }))
}

fn peek_missing(target: &str) -> serde_json::Value {
    serde_json::json!({
        "path": "not found",
        "html": format!("<em>No note found for “{}”.</em>", target.replace('<', "&lt;")),
        "truncated": false,
    })
}

// ---- Phase C: vault workflows ----

/// Open (or create) today's daily note per the vault's daily-notes config.
#[tauri::command]
fn daily_note(root: String) -> Result<String, String> {
    let rootp = Path::new(&root);
    let cfg = to_markdown_mcp::obsidian::config::read_config(rootp).unwrap_or_default();
    let fmt = to_markdown_mcp::obsidian::config::moment_to_chrono(
        cfg.daily_notes_format.as_deref().unwrap_or("YYYY-MM-DD"),
    );
    let name = chrono::Local::now().format(&fmt).to_string();
    let folder = cfg.daily_notes_folder.unwrap_or_default();
    let rel = if folder.is_empty() {
        format!("{}.md", name)
    } else {
        format!("{}/{}.md", folder.trim_end_matches('/'), name)
    };
    let abs = rootp.join(&rel);
    if abs.is_file() {
        return Ok(abs.to_string_lossy().into_owned());
    }
    let v = vault_tools::create_note_from_template(rootp, None, None, true).map_err(|e| e.to_string())?;
    obsidian_vault::invalidate(rootp);
    let rel = v["created"].as_str().ok_or("create returned no path")?;
    Ok(rootp.join(rel).to_string_lossy().into_owned())
}

/// Markdown templates in the vault's configured templates folder.
#[tauri::command]
fn list_templates(root: String) -> Vec<String> {
    let rootp = Path::new(&root);
    let Ok(cfg) = to_markdown_mcp::obsidian::config::read_config(rootp) else { return Vec::new() };
    let Some(folder) = cfg.templates_folder else { return Vec::new() };
    let Ok(entries) = std::fs::read_dir(rootp.join(&folder)) else { return Vec::new() };
    let mut out: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("md") {
                return None;
            }
            let stem = p.file_stem()?.to_string_lossy().into_owned();
            Some(format!("{}/{}", folder.trim_end_matches('/'), stem))
        })
        .collect();
    out.sort();
    out
}

#[tauri::command]
fn new_folder(path: String) -> Result<(), String> {
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())
}

/// Delete a file or directory (frontend confirms first).
#[tauri::command]
fn delete_path(path: String, vault_root: Option<String>) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let result = if p.is_dir() {
        std::fs::remove_dir_all(&p)
    } else {
        std::fs::remove_file(&p)
    };
    if let Some(root) = vault_root {
        obsidian_vault::invalidate(Path::new(&root));
    }
    result.map_err(|e| e.to_string())
}

/// Read the macOS drag pasteboard right after a drop. Browser image drags
/// (Safari/Chrome) reach Tauri with empty `paths` because the "file" is a
/// promise, not a file on disk — but the drag pasteboard still carries the
/// image's URL (and often a plain-text fallback). Returns the first http(s)
/// URL found, if any.
#[cfg(target_os = "macos")]
#[tauri::command]
fn read_drag_pasteboard(app: tauri::AppHandle) -> Result<DragPasteboard, String> {
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        use objc2_app_kit::{
            NSPasteboard, NSPasteboardNameDrag, NSPasteboardTypePNG, NSPasteboardTypeString,
            NSPasteboardTypeTIFF, NSPasteboardTypeURL,
        };
        let pb = unsafe { NSPasteboard::pasteboardWithName(NSPasteboardNameDrag) };
        let mut out = DragPasteboard::default();
        if let Some(types) = pb.types() {
            out.types = types.iter().map(|t| t.to_string()).collect();
        }
        for ty in [unsafe { NSPasteboardTypeURL }, unsafe { NSPasteboardTypeString }] {
            if let Some(s) = pb.stringForType(ty) {
                let s = s.to_string();
                let s = s.trim().to_string();
                if s.starts_with("http://") || s.starts_with("https://") {
                    out.url = Some(s);
                    break;
                }
            }
        }
        // Rendered image data (browsers put a PNG/TIFF of the dragged image on
        // the pasteboard even when the "file" is only a promise).
        for (ty, ext) in
            [(unsafe { NSPasteboardTypePNG }, "png"), (unsafe { NSPasteboardTypeTIFF }, "tiff")]
        {
            if let Some(data) = pb.dataForType(ty) {
                out.image_base64 = Some(base64_encode(&data.to_vec()));
                out.image_ext = Some(ext.into());
                break;
            }
        }
        let _ = tx.send(out);
    })
    .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())
}

#[derive(serde::Serialize, Default, Debug)]
struct DragPasteboard {
    url: Option<String>,
    image_base64: Option<String>,
    image_ext: Option<String>,
    types: Vec<String>,
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { TABLE[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn read_drag_pasteboard() -> Result<DragPasteboard, String> {
    Ok(DragPasteboard::default())
}

fn debug_log_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("debug.log"))
}

/// Append one line to the persistent debug log (for bug reports).
#[tauri::command]
fn debug_log(app: tauri::AppHandle, line: String) -> Result<String, String> {
    use std::io::Write;
    let path = debug_log_file(&app)?;
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    writeln!(f, "{} {}", ts, line).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
fn debug_log_path(app: tauri::AppHandle) -> Result<String, String> {
    debug_log_file(&app).map(|p| p.display().to_string())
}

#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("open").arg("-R").arg(&path).status();
    #[cfg(target_os = "linux")]
    let status = std::process::Command::new("xdg-open")
        .arg(Path::new(&path).parent().unwrap_or(Path::new("/")))
        .status();
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("explorer").arg("/select,").arg(&path).status();
    status.map_err(|e| e.to_string()).and_then(|s| {
        s.success().then_some(()).ok_or_else(|| "could not reveal file".to_string())
    })
}

/// Notes that mention this note's title or aliases in plain text without
/// linking to it (Obsidian's "unlinked mentions").
#[tauri::command]
async fn unlinked_mentions(root: String, path: String) -> Result<serde_json::Value, String> {
    let rootp = PathBuf::from(&root);
    let idx = obsidian_vault::get_index(&rootp).map_err(|e| e.to_string())?;
    let rel = vault_rel(&root, &path).ok_or("File is outside the vault")?;
    let meta = idx.notes.get(&rel).ok_or("Not an indexed note")?;
    let mut needles: Vec<String> = vec![meta.title.to_lowercase()];
    needles.extend(meta.aliases.iter().map(|a| a.to_lowercase()));
    let linked: std::collections::HashSet<&str> = idx
        .backlinks
        .get(&rel)
        .map(|b| b.iter().map(|l| l.from.as_str()).collect())
        .unwrap_or_default();
    let mut hits = Vec::new();
    for other in idx.notes.keys() {
        if other == &rel || linked.contains(other.as_str()) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(rootp.join(other)) else { continue };
        for (i, line) in content.lines().enumerate() {
            let ll = line.to_lowercase();
            if needles.iter().any(|n| ll.contains(n.as_str())) && !line.contains("[[") {
                hits.push(serde_json::json!({
                    "note": other,
                    "line": i + 1,
                    "context": line.trim().chars().take(120).collect::<String>(),
                }));
                break; // one mention per note is enough for the panel
            }
        }
        if hits.len() >= 20 {
            break;
        }
    }
    Ok(serde_json::json!(hits))
}

// ---- Phase E: exports, menu, OS open events ----

#[tauri::command]
async fn export_docx(app: tauri::AppHandle, title: String, md: String) -> Result<Option<String>, String> {
    let Some(dest) = app
        .dialog()
        .file()
        .set_file_name(format!("{}.docx", title.trim_end_matches(".md")))
        .add_filter("Word document", &["docx"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = dest.into_path().map_err(|e| e.to_string())?;
    let bytes = export::markdown_to_docx(&md, title.trim_end_matches(".md"))?;
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
async fn export_rtf(app: tauri::AppHandle, title: String, md: String) -> Result<Option<String>, String> {
    let Some(dest) = app
        .dialog()
        .file()
        .set_file_name(format!("{}.rtf", title.trim_end_matches(".md")))
        .add_filter("Rich Text", &["rtf"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = dest.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, export::markdown_to_rtf(&md, title.trim_end_matches(".md")))
        .map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

/// Files handed to the app by the OS (Finder double-click, CLI args) before
/// the frontend was listening.
#[derive(Default)]
struct PendingOpens(Mutex<Vec<String>>);

#[tauri::command]
fn take_pending_opens(state: tauri::State<PendingOpens>) -> Vec<String> {
    std::mem::take(&mut *state.0.lock().unwrap())
}

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
    let app_menu = Submenu::with_items(app, "toMarkdown", true, &[
        &PredefinedMenuItem::about(app, None, Some(AboutMetadata::default()))?,
        &PredefinedMenuItem::separator(app)?,
        &PredefinedMenuItem::hide(app, None)?,
        &PredefinedMenuItem::quit(app, None)?,
    ])?;
    let file = Submenu::with_items(app, "File", true, &[
        &MenuItem::with_id(app, "open-folder", "Open Folder…", true, Some("CmdOrCtrl+Shift+O"))?,
        &MenuItem::with_id(app, "open-file", "Open File…", true, None::<&str>)?,
        &MenuItem::with_id(app, "new-note", "New Note", true, Some("CmdOrCtrl+N"))?,
        &MenuItem::with_id(app, "daily-note", "Daily Note", true, None::<&str>)?,
        &PredefinedMenuItem::separator(app)?,
        &MenuItem::with_id(app, "export-html", "Export HTML…", true, None::<&str>)?,
        &MenuItem::with_id(app, "export-docx", "Export DOCX…", true, None::<&str>)?,
        &MenuItem::with_id(app, "export-rtf", "Export RTF…", true, None::<&str>)?,
        &MenuItem::with_id(app, "print", "Print / PDF…", true, Some("CmdOrCtrl+P"))?,
    ])?;
    let edit = Submenu::with_items(app, "Edit", true, &[
        &PredefinedMenuItem::undo(app, None)?,
        &PredefinedMenuItem::redo(app, None)?,
        &PredefinedMenuItem::separator(app)?,
        &PredefinedMenuItem::cut(app, None)?,
        &PredefinedMenuItem::copy(app, None)?,
        &PredefinedMenuItem::paste(app, None)?,
        &PredefinedMenuItem::select_all(app, None)?,
    ])?;
    let view = Submenu::with_items(app, "View", true, &[
        &MenuItem::with_id(app, "mode-read", "Reading", true, None::<&str>)?,
        &MenuItem::with_id(app, "mode-live", "Live Editing", true, Some("CmdOrCtrl+E"))?,
        &MenuItem::with_id(app, "mode-split", "Split Source", true, Some("CmdOrCtrl+Shift+E"))?,
        &PredefinedMenuItem::separator(app)?,
        &MenuItem::with_id(app, "zen", "Zen Mode", true, None::<&str>)?,
        &PredefinedMenuItem::separator(app)?,
        &MenuItem::with_id(app, "theme-system", "Theme: System", true, None::<&str>)?,
        &MenuItem::with_id(app, "theme-light", "Theme: Light", true, None::<&str>)?,
        &MenuItem::with_id(app, "theme-dark", "Theme: Dark", true, None::<&str>)?,
        &MenuItem::with_id(app, "theme-sepia", "Theme: Sepia", true, None::<&str>)?,
    ])?;
    let window = Submenu::with_items(app, "Window", true, &[
        &PredefinedMenuItem::minimize(app, None)?,
        &PredefinedMenuItem::maximize(app, None)?,
    ])?;
    Menu::with_items(app, &[&app_menu, &file, &edit, &view, &window])
}

/// Read a small text file (user CSS). Capped at 1MB.
#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    let p = PathBuf::from(&path);
    let size = std::fs::metadata(&p).map(|m| m.len()).map_err(|e| e.to_string())?;
    if size > 1024 * 1024 {
        return Err("File larger than 1MB".to_string());
    }
    std::fs::read_to_string(&p).map_err(|e| e.to_string())
}

#[tauri::command]
async fn pick_folder(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
async fn pick_file(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

/// Result of converting an external file or URL to Markdown.
#[derive(Serialize)]
struct Converted {
    markdown: String,
    suggested_name: String,
    source: String,
}

fn slugify(s: &str) -> String {
    let slug: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let slug = slug.trim().to_string();
    if slug.is_empty() { "Imported".into() } else { slug.chars().take(80).collect() }
}

#[tauri::command]
async fn convert_file_to_markdown(path: String) -> Result<Converted, String> {
    let p = PathBuf::from(&path);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Imported".into());
    let markdown = tauri::async_runtime::spawn_blocking(move || {
        convert_any_to_markdown(&p).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(Converted { markdown, suggested_name: format!("{}.md", slugify(&stem)), source: path })
}

#[tauri::command]
async fn convert_url_to_markdown(url: String) -> Result<Converted, String> {
    use to_markdown_mcp::html_converter;
    use to_markdown_mcp::sources::{fetch_from_source, SourceType};
    let src = SourceType::from_string(&url).map_err(|e| e.to_string())?;
    if !matches!(src, SourceType::Url(_)) {
        return Err("Enter a full http(s):// URL.".into());
    }
    let html = fetch_from_source(&src).await.map_err(|e| e.to_string())?;
    let markdown =
        html_converter::html_to_markdown_with_metadata(&html, true).map_err(|e| e.to_string())?;
    let title = html_converter::extract_html_metadata(&html)
        .ok()
        .and_then(|m| m.get("title").cloned())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| {
            url.split("//").nth(1).and_then(|r| r.split('/').next()).unwrap_or("Imported").into()
        });
    Ok(Converted { markdown, suggested_name: format!("{}.md", slugify(&title)), source: url })
}

/// Write `markdown` into `<root>/Imports/`, deduping the filename derived
/// from `suggested_name` against any existing files there.
fn save_markdown_to_imports(root: &Path, suggested_name: &str, markdown: &str) -> Result<PathBuf, String> {
    let dir = root.join("Imports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let base = suggested_name.trim_end_matches(".md");
    let mut path = dir.join(format!("{}.md", base));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{} {}.md", base, n));
        n += 1;
    }
    std::fs::write(&path, markdown).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Save converted markdown: into `<root>/Imports/` (deduped) or via Save As dialog.
#[tauri::command]
async fn save_import(
    app: tauri::AppHandle,
    root: Option<String>,
    suggested_name: String,
    markdown: String,
    pick: bool,
) -> Result<Option<String>, String> {
    if pick || root.is_none() {
        let Some(dest) = app
            .dialog()
            .file()
            .set_file_name(&suggested_name)
            .add_filter("Markdown", &["md"])
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = dest.into_path().map_err(|e| e.to_string())?;
        std::fs::write(&path, markdown).map_err(|e| e.to_string())?;
        return Ok(Some(path.display().to_string()));
    }
    let path = save_markdown_to_imports(&PathBuf::from(root.unwrap()), &suggested_name, &markdown)?;
    Ok(Some(path.display().to_string()))
}

/// One tab of the Text Analysis overlay: label + Markdown body.
#[derive(Serialize)]
struct MetricsSection {
    label: String,
    markdown: String,
}

/// TUI-style text analysis: word/char/space/token counts (OpenAI exact +
/// Anthropic estimate) with full frequency tables, one section per tab.
#[tauri::command]
async fn text_metrics(content: String) -> Result<Vec<MetricsSection>, String> {
    use to_markdown_mcp::textmetrics::{analyze_text, TokenizerSpec};
    fn freq_table(rows: &[(String, usize)], total: usize) -> String {
        if rows.is_empty() {
            return "(none)\n".into();
        }
        let mut s = String::from("| # | Item | Count | Share |\n| --- | --- | --- | --- |\n");
        for (i, (w, c)) in rows.iter().enumerate() {
            let share = if total > 0 { *c as f64 * 100.0 / total as f64 } else { 0.0 };
            s.push_str(&format!(
                "| {} | `{}` | {} | {:.1}% |\n",
                i + 1,
                w.replace('|', "\\|").replace('`', "'"),
                c,
                share
            ));
        }
        s
    }
    tauri::async_runtime::spawn_blocking(move || {
        let openai = analyze_text(&content, &TokenizerSpec::OpenAi { model: "gpt-4o".into() })
            .map_err(|e| e.to_string())?;
        let anthropic = analyze_text(&content, &TokenizerSpec::Anthropic)
            .map_err(|e| e.to_string())?;
        let mut summary = String::from("| Metric | Value |\n| --- | --- |\n");
        summary.push_str(&format!("| Words | {} |\n", openai.words));
        summary.push_str(&format!("| Distinct words | {} |\n", openai.word_freq.len()));
        summary.push_str(&format!("| Characters | {} |\n", openai.chars));
        summary.push_str(&format!("| Spaces | {} |\n", openai.spaces));
        summary.push_str(&format!(
            "| OpenAI tokens | {}{} |\n",
            openai.tokens,
            if openai.exact { "" } else { " (estimated)" }
        ));
        summary.push_str(&format!("| Anthropic/Claude tokens | ~{} |\n", anthropic.tokens));
        summary.push_str(&format!("| Tokenization | {} |\n", openai.method));
        let sections = vec![
            MetricsSection { label: "Summary".into(), markdown: summary },
            MetricsSection {
                label: format!("Words ({})", openai.word_freq.len()),
                markdown: freq_table(&openai.word_freq, openai.words),
            },
            MetricsSection {
                label: format!("Characters ({})", openai.char_freq.len()),
                markdown: freq_table(&openai.char_freq, openai.chars),
            },
            MetricsSection {
                label: format!("Tokens ({})", openai.token_freq.len()),
                markdown: freq_table(&openai.token_freq, openai.tokens),
            },
        ];
        Ok(sections)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Extensions the converter pipeline handles beyond plain Markdown.
#[tauri::command]
fn is_convertible(path: String) -> bool {
    let ext = Path::new(&path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    matches!(
        ext.as_deref(),
        Some(
            "pdf" | "docx" | "xlsx" | "pptx" | "odt" | "ods" | "odp" | "eml" | "epub" | "html"
                | "htm" | "xml" | "rss" | "atom" | "csv" | "tsv" | "json" | "yaml" | "yml"
                | "toml" | "rst" | "adoc" | "asciidoc" | "org" | "tex" | "webarchive"
        )
    )
}

#[derive(serde::Serialize, Debug)]
struct Span {
    start: usize,
    end: usize,
    kind: String,
}

fn highlight_spans(source: &str) -> Vec<Span> {
    use pulldown_cmark::{Event, Options, Parser, Tag};
    let mut spans: Vec<Span> = Vec::new();
    let mut push = |range: std::ops::Range<usize>, kind: &str| {
        if range.start < range.end {
            spans.push(Span { start: range.start, end: range.end, kind: kind.into() });
        }
    };
    // Frontmatter is not markdown; detect it textually before parsing.
    let mut body_off = 0usize;
    if let Some(rest) = source.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            let after = 4 + end + 4;
            let fm_end = source[after..].find('\n').map(|n| after + n + 1).unwrap_or(source.len());
            push(0..fm_end, "frontmatter");
            body_off = fm_end;
        }
    }
    let body = &source[body_off..];
    // Wikilinks: pulldown-cmark doesn't know them; linear scan.
    let mut i = 0;
    while let Some(open) = body[i..].find("[[") {
        let s = i + open;
        match body[s..].find("]]") {
            Some(close) => {
                push(body_off + s..body_off + s + close + 2, "wikilink");
                i = s + close + 2;
            }
            None => break,
        }
    }
    for (event, range) in Parser::new_ext(body, Options::all()).into_offset_iter() {
        let r = body_off + range.start..body_off + range.end;
        match event {
            Event::Start(Tag::Heading { .. }) => push(r, "heading"),
            Event::Start(Tag::Emphasis) => push(r, "emphasis"),
            Event::Start(Tag::Strong) => push(r, "strong"),
            Event::Start(Tag::CodeBlock(_)) => push(r, "codeblock"),
            Event::Start(Tag::Link { .. }) | Event::Start(Tag::Image { .. }) => push(r, "link"),
            Event::Start(Tag::BlockQuote(_)) => push(r, "blockquote"),
            Event::Code(_) => push(r, "code"),
            Event::Start(Tag::Item) => {
                let text = &body[range.start..range.end];
                let marker_len = text.find(|c: char| !"-*+0123456789. \t".contains(c)).unwrap_or(0);
                let end = (r.start + marker_len).min(r.end);
                push(r.start..end, "list_marker");
            }
            _ => {}
        }
    }
    spans.sort_by_key(|s| (s.start, s.end));
    spans
}

#[tauri::command]
fn highlight_markdown(source: String) -> Vec<Span> {
    highlight_spans(&source)
}

// ---- Word Graph (Phase 10) ----

use crate::word_graph::{queries, db::WordGraphDb};

#[derive(Serialize)]
struct WordGraphNode {
    id: i32,
    word: String,
    frequency: i32,
}

#[derive(Serialize)]
struct WordGraphLink {
    source: i32,
    target: i32,
    weight: i32,
}

#[derive(Serialize)]
struct WordGraphResponse {
    nodes: Vec<WordGraphNode>,
    links: Vec<WordGraphLink>,
    last_updated: Option<String>,
}

#[tauri::command]
fn word_graph_data(root: String) -> Result<WordGraphResponse, String> {
    let vault_path = std::path::Path::new(&root);
    eprintln!("[WORD_GRAPH] Fetching graph data for vault: {}", vault_path.display());

    let db = WordGraphDb::new(vault_path).map_err(|e| {
        eprintln!("[WORD_GRAPH] Failed to open database for data fetch: {}", e);
        e.to_string()
    })?;

    // Adaptive word limit
    let vault_size = count_markdown_files(vault_path).unwrap_or(0);
    let word_limit = std::cmp::min(100, std::cmp::max(1, vault_size / 10));

    let words = queries::get_top_words(&db, word_limit).map_err(|e| e.to_string())?;

    let word_ids: Vec<i32> = words.iter()
        .map(|(w, _)| {
            db.conn().query_row("SELECT id FROM words WHERE word = ?1", [w], |row| row.get(0))
        })
        .filter_map(|r| r.ok())
        .collect();

    let nodes: Vec<WordGraphNode> = words.into_iter().enumerate().map(|(idx, (word, freq))| {
        WordGraphNode {
            id: idx as i32,
            word,
            frequency: freq,
        }
    }).collect();

    let pairs = queries::get_word_pairs_for_graph(&db, &word_ids, 2)
        .map_err(|e| e.to_string())?;

    let links: Vec<WordGraphLink> = pairs.into_iter().filter_map(|(w1_id, w2_id, count)| {
        let source = word_ids.iter().position(|&id| id == w1_id)? as i32;
        let target = word_ids.iter().position(|&id| id == w2_id)? as i32;
        Some(WordGraphLink {
            source,
            target,
            weight: count,
        })
    }).collect();

    let last_updated = queries::get_last_updated_timestamp(&db).ok().flatten();
    eprintln!("[WORD_GRAPH] Successfully fetched {} nodes and {} links",
        nodes.len(), links.len());
    Ok(WordGraphResponse {
        nodes,
        links,
        last_updated,
    })
}

#[tauri::command]
fn index_vault_words(root: String) -> Result<(), String> {
    let vault_path = std::path::Path::new(&root);
    eprintln!("[WORD_GRAPH] Starting full index of vault: {}", vault_path.display());

    // Spawn indexing in background
    tauri::async_runtime::spawn_blocking(move || {
        let vault_path = std::path::Path::new(&root);
        match WordGraphDb::new(vault_path) {
            Ok(db) => {
                eprintln!("[WORD_GRAPH] Database opened, beginning indexing...");
                match crate::word_graph::indexer::index_vault_full(&db, vault_path) {
                    Ok(_) => eprintln!("[WORD_GRAPH] Full index completed successfully"),
                    Err(e) => eprintln!("[WORD_GRAPH] Full index failed: {}", e),
                }
            }
            Err(e) => eprintln!("[WORD_GRAPH] Failed to open database: {}", e),
        }
    });

    Ok(())
}

#[tauri::command]
fn delta_index_vault_words(root: String, changed_files: Vec<String>) -> Result<(), String> {
    // Spawn in background
    tauri::async_runtime::spawn_blocking(move || {
        let vault_path = std::path::PathBuf::from(&root);
        let file_paths: Vec<std::path::PathBuf> = changed_files.iter()
            .map(|f| vault_path.join(f))
            .collect();
        let db = WordGraphDb::new(&vault_path).map_err(|e| e.to_string())?;
        crate::word_graph::indexer::index_vault_delta(&db, &vault_path, &file_paths)
            .map_err(|e| e.to_string())
    });

    Ok(())
}

#[tauri::command]
fn word_graph_notes(root: String, word: String) -> Result<Vec<String>, String> {
    let vault_path = std::path::Path::new(&root);
    let db = WordGraphDb::new(vault_path).map_err(|e| e.to_string())?;
    let notes = queries::get_notes_for_word(&db, &word).map_err(|e| e.to_string())?;
    Ok(notes)
}

fn count_markdown_files(path: &std::path::Path) -> std::io::Result<usize> {
    let mut count = 0;
    fn walk(path: &std::path::Path, count: &mut usize) -> std::io::Result<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && !path.file_name().unwrap_or_default().to_string_lossy().starts_with('.') {
                walk(&path, count)?;
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                *count += 1;
            }
        }
        Ok(())
    }
    walk(path, &mut count)?;
    Ok(count)
}

fn main() {
    use tauri::Manager;

    // Initialize logging to default location
    let default_logs_root = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
    let _ = logging::init_logging(&default_logs_root);

    // Initialize default user preferences for VaultViewerState
    let default_prefs = UserPreferences {
        tab_mode: TabMode::Multi,
        theme: "light".to_string(),
        auto_restore_tabs: true,
        recycle_retention_days: 180,
        auto_save: true,
        show_toast: true,
        zoom_behavior: ZoomBehavior::ResetPerImage,
        mouse_wheel_zoom: true,
    };
    let vault_viewer_state = VaultViewerState::new(default_prefs);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(WatchState::default())
        .manage(PendingOpens::default())
        .manage(Mutex::new(vault_viewer_state))
        .manage(Mutex::new(HashMap::<String, ZoomCalculator>::new()))
        .menu(build_menu)
        .on_menu_event(|app, event| {
            let _ = app.emit("menu-action", event.id().0.clone());
        })
        .setup(|app| {
            // Files passed as CLI arguments (Linux/Windows file associations).
            let args: Vec<String> = std::env::args()
                .skip(1)
                .filter(|a| Path::new(a).is_file())
                .collect();
            if !args.is_empty() {
                app.state::<PendingOpens>().0.lock().unwrap().extend(args);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_tree, open_file, view_binary, pick_folder, pick_file,
            watch_tree, watch_file, export_html, read_text_file,
            note_info, vault_overview, vault_search, vault_tasks,
            resolve_wikilink, graph_data, quick_list, wikilink_complete,
            read_source, save_file, render_markdown, toggle_task,
            create_note, rename_note, set_frontmatter, paste_image, store_attachment, store_url_attachment, tag_list,
            scan_external_links, localize_link,
            related_notes, semantic_search, set_api_key, ai_action, syntax_css,
            doc_stats, peek_note,
            daily_note, list_templates, new_folder, delete_path,
            reveal_in_finder, unlinked_mentions, render_blocks,
            export_docx, export_rtf, take_pending_opens,
            convert_file_to_markdown, convert_url_to_markdown, save_import, is_convertible,
            text_metrics, highlight_markdown, debug_log, debug_log_path, read_drag_pasteboard,
            word_graph_data, index_vault_words, delta_index_vault_words, word_graph_notes,
            commands::file_ops::duplicate_file, commands::file_ops::create_markdown_note,
            commands::preview::get_file_preview,
            commands::recent::get_recent_files, commands::recent::update_recent_file,
            commands::search::search_files,
            commands::tab_commands::add_tab, commands::tab_commands::close_tab,
            commands::tab_commands::set_active_tab, commands::tab_commands::back_button,
            commands::tab_commands::get_tabs, commands::tab_commands::set_vault_root,
            commands::tab_commands::load_saved_tabs,
            commands::recycle_commands::delete_file, commands::recycle_commands::restore_file,
            commands::recycle_commands::permanently_delete, commands::recycle_commands::get_deleted_files,
            commands::recycle_commands::empty_recycle_bin,
            commands::clipboard_commands::copy_file,
            commands::settings_commands::get_preferences, commands::settings_commands::update_preferences,
            commands::image_commands::zoom_in, commands::image_commands::zoom_out,
            commands::image_commands::reset_zoom, commands::image_commands::fit_to_window,
            commands::image_commands::pan
        ])
        .build(tauri::generate_context!())
        .expect("error while building toMarkdown Viewer");
    app.run(|app_handle, event| {
        // Finder double-click / "Open With" on macOS.
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Opened { urls } = event {
            let paths: Vec<String> = urls
                .iter()
                .filter_map(|u| u.to_file_path().ok())
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            if !paths.is_empty() {
                app_handle
                    .state::<PendingOpens>()
                    .0
                    .lock()
                    .unwrap()
                    .extend(paths.clone());
                let _ = app_handle.emit("os-open-file", paths);
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app_handle, event);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_markdown_spans_basic() {
        let spans = highlight_spans("# Title\n\nsome **bold** and `code`\n");
        let kind_at = |off: usize| spans.iter().find(|s| s.start <= off && off < s.end).map(|s| s.kind.as_str());
        assert_eq!(kind_at(0), Some("heading"));      // '#'
        assert_eq!(kind_at(14), Some("strong"));      // inside **bold**
        assert_eq!(kind_at(28), Some("code"));        // inside `code`
        assert!(spans.iter().all(|s| s.start < s.end));
    }

    #[test]
    fn highlight_markdown_spans_wikilink_and_fence() {
        let src = "see [[Other Note]]\n\n```rust\nfn x() {}\n```\n";
        let spans = highlight_spans(src);
        assert!(spans.iter().any(|s| s.kind == "wikilink" && &src[s.start..s.end] == "[[Other Note]]"));
        assert!(spans.iter().any(|s| s.kind == "codeblock"));
    }

    #[test]
    fn highlight_markdown_spans_frontmatter() {
        let src = "---\ntitle: X\n---\n# H\n";
        let spans = highlight_spans(src);
        assert!(spans.iter().any(|s| s.kind == "frontmatter" && s.start == 0));
    }

    fn fixture_vault() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/mini_vault")
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn wikilinks_become_clickable_anchors() {
        let root = PathBuf::from(fixture_vault());
        let opts = RenderOpts { file_dir: Some(&root), vault_root: Some(&root) };
        let html = render_note("See [[Note B|the second]] and [[Note A#Heading]].", &opts, 0);
        assert!(html.contains(r#"href="wikilink:Note%20B""#), "html: {}", html);
        assert!(html.contains(">the second<"));
        assert!(html.contains(r#"href="wikilink:Note%20A#Heading""#));
    }

    #[test]
    fn resolve_wikilink_finds_fixture_note() {
        let root = fixture_vault();
        let resolved = resolve_wikilink(root.clone(), "Note B".into(), format!("{}/Note A.md", root)).unwrap();
        assert!(resolved.ends_with("Note B.md"));
        assert!(resolve_wikilink(root, "No Such Note".into(), String::new()).is_err());
    }

    #[test]
    fn graph_data_links_fixture_notes() {
        let root = fixture_vault();
        let g = tauri::async_runtime::block_on(graph_data(root.clone(), None)).unwrap();
        assert!(!g["nodes"].as_array().unwrap().is_empty());
        assert!(!g["links"].as_array().unwrap().is_empty());
        // Local graph keeps only the focused note's neighborhood.
        let local = tauri::async_runtime::block_on(graph_data(root.clone(), Some(format!("{}/Note A.md", root)))).unwrap();
        assert!(local["nodes"].as_array().unwrap().len() <= g["nodes"].as_array().unwrap().len());
    }

    #[test]
    fn note_info_returns_properties_and_backlinks() {
        let root = fixture_vault();
        let info = note_info(root.clone(), format!("{}/Note A.md", root)).unwrap();
        assert_eq!(info["note"]["title"], "Note A");
        assert!(info["backlinks"]["backlink_count"].as_u64().is_some());
    }

    fn temp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("gui_edit_test_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A throwaway vault directory for tests that need to write into a
    /// vault root (unlike `fixture_vault()`, which points at the
    /// read-only checked-in fixtures). Mirrors `temp_dir()`'s pattern of
    /// a std::env::temp_dir subdir keyed by pid, made unique per-call via
    /// a name suffix so concurrent tests don't collide.
    struct TempVaultDir(PathBuf);

    impl TempVaultDir {
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempVaultDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temp_vault(name: &str) -> TempVaultDir {
        let d = std::env::temp_dir().join(format!("gui_store_attachment_test_{}_{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        TempVaultDir(d)
    }

    #[test]
    fn store_attachment_copies_and_dedupes() {
        let root = temp_vault("copies_and_dedupes");
        let src = root.path().join("src img.png");
        std::fs::write(&src, b"fakepng").unwrap();
        let a = store_attachment_impl(root.path(), &src).unwrap();
        assert!(a.embed.starts_with("![["), "png should embed: {}", a.embed);
        let b = store_attachment_impl(root.path(), &src).unwrap();
        assert_ne!(a.name, b.name, "second copy must dedupe filename");
        // both files exist in the attachment dir
        let dir = attachment_dir(root.path()).unwrap();
        assert!(dir.join(&a.name).exists() && dir.join(&b.name).exists());
    }

    #[test]
    fn store_attachment_non_image_links_instead_of_embeds() {
        let root = temp_vault("non_image");
        let src = root.path().join("doc.pdf");
        std::fs::write(&src, b"%PDF").unwrap();
        let a = store_attachment_impl(root.path(), &src).unwrap();
        assert!(a.embed.starts_with("[[") && !a.embed.starts_with("![["));
    }

    #[test]
    fn store_attachment_tiff_embeds() {
        let root = temp_vault("tiff_embed");
        let src = root.path().join("image.tiff");
        std::fs::write(&src, b"fakeTIFF").unwrap();
        let a = store_attachment_impl(root.path(), &src).unwrap();
        assert!(a.embed.starts_with("![["), "tiff should embed: {}", a.embed);
        // Also test lowercase tif extension
        let src2 = root.path().join("image2.tif");
        std::fs::write(&src2, b"fakeTIFF").unwrap();
        let b = store_attachment_impl(root.path(), &src2).unwrap();
        assert!(b.embed.starts_with("![["), "tif should embed: {}", b.embed);
    }

    #[test]
    fn store_attachment_missing_source_errors() {
        let root = temp_vault("missing_source");
        assert!(store_attachment_impl(root.path(), Path::new("/nope/x.png")).is_err());
    }

    #[test]
    fn url_attachment_name_derivation() {
        assert_eq!(url_attachment_name("https://x.com/a/photo.png?w=2", None, None), "photo.png");
        assert_eq!(url_attachment_name("https://x.com/img", Some("image/jpeg"), None), "img.jpg");
        let n = url_attachment_name("https://x.com/", None, None);
        assert!(!n.is_empty() && !n.contains('/'));
    }

    #[test]
    fn url_attachment_name_keeps_original_names() {
        // Percent-encoded original filename is decoded.
        assert_eq!(
            url_attachment_name("https://x.com/my%20vacation%20photo.png", None, None),
            "my vacation photo.png"
        );
        // Content-Disposition name wins over the URL path.
        assert_eq!(
            url_attachment_name("https://cdn.x.com/asset?id=42", Some("image/png"), Some("original.png")),
            "original.png"
        );
        // Extension-less CDN URL gets the content-type extension.
        assert_eq!(url_attachment_name("https://cdn.x.com/abc123", Some("image/webp"), None), "abc123.webp");
    }

    #[test]
    fn save_is_atomic_and_roundtrips() {
        let dir = temp_dir();
        let f = dir.join("note.md");
        save_file(f.to_string_lossy().into(), "# Hi\n".into(), None).unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "# Hi\n");
        assert!(!f.with_extension("tomarkdown.tmp").exists());
    }

    #[test]
    fn wikilink_complete_ranks_prefix_first() {
        let root = fixture_vault();
        // "note" title-prefixes "Note A" (x2) and "Note B", and is a
        // non-prefix substring of the fixture note "Another Note" — so this
        // exercises both rank 0 (prefix) and rank 1 (substring) results.
        let hits = wikilink_matches(Path::new(&root), "note").unwrap();
        assert!(!hits.is_empty());
        let labels: Vec<String> = hits.iter().map(|h| h.label.to_lowercase()).collect();
        // Guard against fixture drift silently making this test vacuous.
        assert!(
            labels.iter().any(|l| !l.starts_with("note")),
            "expected at least one non-prefix (substring) match in fixtures; got {:?}",
            labels
        );
        let fs = labels.iter().position(|l| !l.starts_with("note")).unwrap();
        assert!(labels[..fs].iter().all(|l| l.starts_with("note")));
        assert!(labels[fs..].iter().all(|l| !l.starts_with("note")));
        assert!(labels.contains(&"another note".to_string()));
    }

    #[test]
    fn wikilink_complete_empty_prefix_lists_up_to_20() {
        let root = fixture_vault();
        let hits = wikilink_matches(Path::new(&root), "").unwrap();
        assert!(!hits.is_empty());
        assert!(hits.len() <= 20);
    }

    #[test]
    fn wikilink_complete_finds_alias_only_matches() {
        let root = fixture_vault();
        // "Note A" has alias "First Note"; no fixture title contains "first",
        // so this query only matches via rank-2 alias-substring.
        let hits = wikilink_matches(Path::new(&root), "first").unwrap();
        assert!(
            hits.iter().any(|h| h.label == "Note A"),
            "expected alias-only match for 'Note A'; got {:?}",
            hits.iter().map(|h| &h.label).collect::<Vec<_>>()
        );
        let labels: Vec<String> = hits.iter().map(|h| h.label.to_lowercase()).collect();
        assert!(
            labels.iter().all(|l| !l.starts_with("first") && !l.contains("first")),
            "no fixture title should contain 'first'; got {:?}",
            labels
        );
    }

    #[test]
    fn toggle_task_flips_the_right_checkbox() {
        let dir = temp_dir();
        let f = dir.join("tasks.md");
        std::fs::write(&f, "- [ ] first\ntext\n- [x] second\n  - [ ] nested\n").unwrap();
        let p = f.to_string_lossy().into_owned();
        toggle_task(p.clone(), 1, None).unwrap();
        assert!(std::fs::read_to_string(&f).unwrap().contains("- [ ] second"));
        toggle_task(p.clone(), 2, None).unwrap();
        assert!(std::fs::read_to_string(&f).unwrap().contains("  - [x] nested"));
        assert!(toggle_task(p, 9, None).is_err());
    }

    #[test]
    fn set_frontmatter_replaces_only_the_yaml_block() {
        let dir = temp_dir();
        let f = dir.join("fm.md");
        std::fs::write(&f, "---\nstatus: old\n---\n# Body\n").unwrap();
        let p = f.to_string_lossy().into_owned();
        set_frontmatter(p.clone(), "status: new\ntags: [a]".into(), None).unwrap();
        let s = std::fs::read_to_string(&f).unwrap();
        assert!(s.contains("status: new") && s.contains("# Body"));
        // Invalid YAML is rejected, file untouched.
        assert!(set_frontmatter(p.clone(), "not: [valid".into(), None).is_err());
        // Empty removes the block.
        set_frontmatter(p, String::new(), None).unwrap();
        assert!(std::fs::read_to_string(&f).unwrap().starts_with("# Body"));
    }

    #[test]
    fn base64_decodes() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert!(base64_decode("!!!").is_none());
    }

    #[test]
    fn peek_note_answers_for_every_fixture_wikilink() {
        use to_markdown_mcp::obsidian::wikilink;
        let root = fixture_vault();
        let rootp = PathBuf::from(&root);
        let mut checked = 0;
        for entry in std::fs::read_dir(&rootp).unwrap().flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let content = std::fs::read_to_string(&p).unwrap();
            for link in wikilink::parse_wikilinks(&content) {
                let v = tauri::async_runtime::block_on(peek_note(
                    root.clone(),
                    link.target.clone(),
                    Some(p.to_string_lossy().into_owned()),
                ))
                .unwrap_or_else(|e| panic!("peek failed for [[{}]] in {}: {}", link.target, p.display(), e));
                assert!(
                    !v["html"].as_str().unwrap_or_default().trim().is_empty(),
                    "empty preview for [[{}]] in {}",
                    link.target,
                    p.display()
                );
                checked += 1;
            }
        }
        assert!(checked >= 8, "expected to exercise several links, got {}", checked);
    }

    #[test]
    fn daily_note_creates_then_reuses() {
        // Work on a copy so the fixture vault stays pristine.
        let src = PathBuf::from(fixture_vault());
        let dst = std::env::temp_dir().join(format!("gui_daily_test_{}", std::process::id()));
        std::fs::remove_dir_all(&dst).ok();
        fn cp(a: &Path, b: &Path) {
            std::fs::create_dir_all(b).unwrap();
            for e in std::fs::read_dir(a).unwrap().flatten() {
                let t = b.join(e.file_name());
                if e.path().is_dir() { cp(&e.path(), &t); } else { std::fs::copy(e.path(), &t).unwrap(); }
            }
        }
        cp(&src, &dst);
        let root = dst.to_string_lossy().into_owned();
        let first = daily_note(root.clone()).unwrap();
        assert!(std::path::Path::new(&first).is_file());
        let second = daily_note(root).unwrap();
        assert_eq!(first, second, "existing daily note should be reused");
        std::fs::remove_dir_all(&dst).ok();
    }

    #[test]
    fn templates_listed_from_vault_config() {
        let templates = list_templates(fixture_vault());
        assert!(!templates.is_empty(), "fixture vault has a templates folder");
        assert!(templates.iter().all(|t| t.contains('/')));
    }

    #[test]
    fn unlinked_mentions_exclude_linkers() {
        let root = fixture_vault();
        let hits = tauri::async_runtime::block_on(unlinked_mentions(
            root.clone(),
            format!("{}/Note A.md", root),
        ))
        .unwrap();
        // Every hit must be plain-text (mentioning without a wikilink).
        for h in hits.as_array().unwrap() {
            assert!(!h["context"].as_str().unwrap().contains("[["));
        }
    }

    #[test]
    fn canvas_files_render() {
        let root = fixture_vault();
        let r = tauri::async_runtime::block_on(open_file(format!("{}/Board.canvas", root), Some(root))).unwrap();
        assert!(!r.html.trim().is_empty());
    }

    /// Sync test wrapper mirroring the `tauri::async_runtime::block_on`
    /// pattern used elsewhere in this file (e.g. `unlinked_mentions`,
    /// `canvas_files_render`) to drive an async command from a plain test.
    fn localize_link_blocking(root: &Path, note: &Path, link: &str, action: &str) -> Result<String, String> {
        tauri::async_runtime::block_on(localize_link_impl(root, note, link, action))
    }

    #[test]
    fn scan_external_links_finds_targets() {
        let root = temp_vault("scan_external_links");
        let note = root.path().join("Ext.md");
        std::fs::write(
            &note,
            "![a](https://x.com/p.png)\n[b](https://x.com/page)\n[c](file:///tmp/d.pdf)\n[[Note A]]\n![local](local.png)\n[dup](https://x.com/page)\n",
        )
        .unwrap();
        let links = scan_external_links_impl(root.path(), &note).unwrap();
        let kinds: Vec<(&str, &str)> = links.iter().map(|l| (l.link.as_str(), l.kind.as_str())).collect();
        assert!(kinds.contains(&("https://x.com/p.png", "image_url")));
        assert!(kinds.contains(&("https://x.com/page", "url")));
        assert!(kinds.contains(&("file:///tmp/d.pdf", "file")));
        assert_eq!(
            links.iter().filter(|l| l.link == "https://x.com/page").count(),
            1,
            "dedup identical targets"
        );
        assert!(
            !kinds.iter().any(|(l, _)| l.contains("Note A") || l.contains("local.png")),
            "vault-internal links excluded"
        );
    }

    #[test]
    fn localize_link_store_rewrites_all_occurrences_of_file_url() {
        let root = temp_vault("localize_link_store");
        let src = root.path().join("att.pdf");
        std::fs::write(&src, b"%PDF").unwrap();
        let file_url = format!("file://{}", src.display());
        let note = root.path().join("Loc.md");
        std::fs::write(&note, format!("[x]({0})\ntext\n[y]({0})\n", file_url)).unwrap();
        let updated = localize_link_blocking(root.path(), &note, &file_url, "store").unwrap();
        assert!(!updated.contains(&file_url), "no occurrence of the old target remains");
        assert_eq!(updated.matches("[[att").count(), 2, "both occurrences rewritten to the same local target");
        assert_eq!(std::fs::read_to_string(&note).unwrap(), updated, "note saved");
    }

    #[test]
    fn localize_link_store_rewrites_title_bearing_link() {
        let root = temp_vault("localize_link_title");
        let src = root.path().join("att.pdf");
        std::fs::write(&src, b"%PDF").unwrap();
        let file_url = format!("file://{}", src.display());
        let note = root.path().join("Loc.md");
        std::fs::write(&note, format!("[Site]({} \"My Title\")\n", file_url)).unwrap();
        let updated = localize_link_blocking(root.path(), &note, &file_url, "store").unwrap();
        assert!(!updated.contains(&file_url), "no occurrence of the old target remains");
        assert!(!updated.contains("My Title"), "the title-bearing construct must be fully replaced");
        assert!(updated.contains("[[att"), "rewritten to a local wikilink: {}", updated);
    }

    #[test]
    fn localize_link_absent_target_errors_without_side_effects() {
        let root = temp_vault("localize_link_absent");
        let note = root.path().join("Loc.md");
        std::fs::write(&note, "no external links here\n").unwrap();
        let missing = "file:///tmp/does-not-appear.pdf";
        let err = localize_link_blocking(root.path(), &note, missing, "store").unwrap_err();
        assert!(err.contains("not found"), "unexpected error: {}", err);
        assert!(
            !root.path().join("does-not-appear.pdf").exists(),
            "no attachment should have been created"
        );
    }
}
