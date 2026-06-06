//! Lattice desktop app backend. Thin Tauri command layer over `lattice-core`.
//! The vault is opened lazily (picked by the user or restored from config /
//! `LATTICE_ROOT`) and held behind a Mutex, since a rusqlite Connection is Send
//! but not Sync. The vault folder is never written into — the index db lives in
//! the app cache dir.

use lattice_core::{
    edit::{RawNote, WriteOutcome},
    model::{Edge, Node},
    tree::TreeEntry,
    Vault,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

struct OpenVault {
    path: String,
    vault: Vault,
}

struct AppState(Mutex<Option<OpenVault>>);

#[derive(Serialize)]
struct VaultInfo {
    path: String,
    name: String,
}

#[derive(Default, Serialize, Deserialize)]
struct Config {
    last_vault: Option<String>,
    #[serde(default)]
    recents: Vec<String>,
}

fn vault_info(path: &str) -> VaultInfo {
    let name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    VaultInfo {
        path: path.to_string(),
        name,
    }
}

fn config_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("config.json"))
}

fn load_config(app: &AppHandle) -> Config {
    config_file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(app: &AppHandle, cfg: &Config) {
    if let Some(p) = config_file(app) {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(cfg) {
            let _ = std::fs::write(p, s);
        }
    }
}

/// Derived index db (rebuilt on open); kept out of the vault folder.
fn db_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_cache_dir()
        .ok()
        .map(|d| {
            let _ = std::fs::create_dir_all(&d);
            d.join("index.db")
        })
        .unwrap_or_else(|| PathBuf::from("lattice.db"))
}

/// Run `f` against the open vault, or return a "no vault" error.
fn with_vault<T, E: std::fmt::Display>(
    state: &State<AppState>,
    f: impl FnOnce(&mut Vault) -> Result<T, E>,
) -> Result<T, String> {
    let mut guard = state.0.lock().unwrap();
    let ov = guard
        .as_mut()
        .ok_or_else(|| "no vault open".to_string())?;
    f(&mut ov.vault).map_err(|e| e.to_string())
}

// --- vault lifecycle ---------------------------------------------------------

#[tauri::command]
fn current_vault(state: State<AppState>) -> Option<VaultInfo> {
    state
        .0
        .lock()
        .unwrap()
        .as_ref()
        .map(|ov| vault_info(&ov.path))
}

#[tauri::command]
fn recents(app: AppHandle) -> Vec<String> {
    load_config(&app).recents
}

/// Native folder picker (runs off the main thread, so blocking is fine here).
#[tauri::command]
fn pick_vault(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|fp| fp.as_path().map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
fn open_vault(app: AppHandle, state: State<AppState>, path: String) -> Result<VaultInfo, String> {
    let vault = Vault::open(&PathBuf::from(&path), &db_path(&app), ".aiignore")
        .map_err(|e| e.to_string())?;
    *state.0.lock().unwrap() = Some(OpenVault {
        path: path.clone(),
        vault,
    });

    let mut cfg = load_config(&app);
    cfg.last_vault = Some(path.clone());
    cfg.recents.retain(|p| p != &path);
    cfg.recents.insert(0, path.clone());
    cfg.recents.truncate(8);
    save_config(&app, &cfg);

    Ok(vault_info(&path))
}

// --- queries / edits ---------------------------------------------------------

#[tauri::command]
fn tree(state: State<AppState>) -> Result<Vec<TreeEntry>, String> {
    with_vault(&state, |v| v.tree())
}

#[tauri::command]
fn render(state: State<AppState>, note: String) -> Result<String, String> {
    with_vault(&state, |v| v.render(&note))
}

#[tauri::command]
fn read_raw(state: State<AppState>, note: String) -> Result<RawNote, String> {
    with_vault(&state, |v| v.read_raw(&note))
}

#[tauri::command]
fn save(
    state: State<AppState>,
    note: String,
    content: String,
    expected_hash: String,
) -> Result<WriteOutcome, String> {
    with_vault(&state, |v| v.save(&note, &content, &expected_hash))
}

#[tauri::command]
fn backlinks(state: State<AppState>, note: String) -> Result<Vec<Edge>, String> {
    with_vault(&state, |v| v.backlinks(&note))
}

#[tauri::command]
fn links(state: State<AppState>, note: String) -> Result<Vec<Edge>, String> {
    with_vault(&state, |v| v.links(&note))
}

#[tauri::command]
fn orphans(state: State<AppState>) -> Result<Vec<Node>, String> {
    with_vault(&state, |v| v.orphans(None, 5000))
}

#[tauri::command]
fn broken_links(state: State<AppState>) -> Result<Vec<Edge>, String> {
    with_vault(&state, |v| v.broken_links(None, 5000))
}

#[tauri::command]
fn search(state: State<AppState>, text: String) -> Result<Vec<Node>, String> {
    with_vault(&state, |v| v.search(&text, None, 50))
}

#[tauri::command]
fn query(state: State<AppState>, filters: Vec<String>) -> Result<Vec<Node>, String> {
    let pairs: Vec<(String, String)> = filters
        .iter()
        .filter_map(|f| f.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
        .collect();
    let refs: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    with_vault(&state, |v| v.query(&refs, None))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Env override (used by `npm run tauri dev` / scripts) takes precedence.
    let initial = std::env::var("LATTICE_ROOT").ok().and_then(|root| {
        let db = std::env::var("LATTICE_DB").unwrap_or_else(|_| "lattice.db".into());
        Vault::open(&PathBuf::from(&root), &PathBuf::from(db), ".aiignore")
            .ok()
            .map(|vault| OpenVault { path: root, vault })
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState(Mutex::new(initial)))
        .setup(|app| {
            // No env vault: restore the last vault from config if it still opens.
            let state = app.state::<AppState>();
            if state.0.lock().unwrap().is_none() {
                if let Some(path) = load_config(app.handle()).last_vault {
                    if let Ok(vault) =
                        Vault::open(&PathBuf::from(&path), &db_path(app.handle()), ".aiignore")
                    {
                        *state.0.lock().unwrap() = Some(OpenVault { path, vault });
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            current_vault,
            recents,
            pick_vault,
            open_vault,
            tree,
            render,
            read_raw,
            save,
            backlinks,
            links,
            orphans,
            broken_links,
            search,
            query
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
