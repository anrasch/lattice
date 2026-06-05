//! Lattice desktop app backend. Thin Tauri command layer over `lattice-core`.
//! The vault is opened once at startup (root + db from env) and guarded by a
//! Mutex, since a rusqlite Connection is Send but not Sync.

use lattice_core::{
    edit::{RawNote, WriteOutcome},
    model::{Edge, Node},
    tree::TreeEntry,
    Vault,
};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

struct AppVault(Mutex<Vault>);

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
fn tree(state: State<AppVault>) -> Result<Vec<TreeEntry>, String> {
    state.0.lock().unwrap().tree().map_err(err)
}

#[tauri::command]
fn render(state: State<AppVault>, note: String) -> Result<String, String> {
    state.0.lock().unwrap().render(&note).map_err(err)
}

#[tauri::command]
fn read_raw(state: State<AppVault>, note: String) -> Result<RawNote, String> {
    state.0.lock().unwrap().read_raw(&note).map_err(err)
}

#[tauri::command]
fn save(
    state: State<AppVault>,
    note: String,
    content: String,
    expected_hash: String,
) -> Result<WriteOutcome, String> {
    state
        .0
        .lock()
        .unwrap()
        .save(&note, &content, &expected_hash)
        .map_err(err)
}

#[tauri::command]
fn backlinks(state: State<AppVault>, note: String) -> Result<Vec<Edge>, String> {
    state.0.lock().unwrap().backlinks(&note).map_err(err)
}

#[tauri::command]
fn links(state: State<AppVault>, note: String) -> Result<Vec<Edge>, String> {
    state.0.lock().unwrap().links(&note).map_err(err)
}

#[tauri::command]
fn orphans(state: State<AppVault>) -> Result<Vec<Node>, String> {
    state.0.lock().unwrap().orphans(None, 5000).map_err(err)
}

#[tauri::command]
fn broken_links(state: State<AppVault>) -> Result<Vec<Edge>, String> {
    state.0.lock().unwrap().broken_links(None, 5000).map_err(err)
}

#[tauri::command]
fn search(state: State<AppVault>, text: String) -> Result<Vec<Node>, String> {
    state.0.lock().unwrap().search(&text, None, 50).map_err(err)
}

#[tauri::command]
fn query(state: State<AppVault>, filters: Vec<String>) -> Result<Vec<Node>, String> {
    let pairs: Vec<(String, String)> = filters
        .iter()
        .filter_map(|f| f.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
        .collect();
    let refs: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    state.0.lock().unwrap().query(&refs, None).map_err(err)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let root = std::env::var("LATTICE_ROOT").unwrap_or_else(|_| ".".into());
    let db = std::env::var("LATTICE_DB").unwrap_or_else(|_| "lattice.db".into());
    let vault = Vault::open(&PathBuf::from(root), &PathBuf::from(db), ".aiignore")
        .expect("failed to open vault");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppVault(Mutex::new(vault)))
        .invoke_handler(tauri::generate_handler![
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
