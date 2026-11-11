use super::RuleEdit;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

/// Separate from `formatting_rules.json`, so importing or resetting the
/// user's formatting rules never touches the medical rule edits.
const STORE_FILE: &str = "medical_rules.json";
const EDITS_KEY: &str = "edits";

pub fn load(app: &AppHandle) -> Result<Vec<RuleEdit>, String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    match store.get(EDITS_KEY) {
        Some(value) => serde_json::from_value(value)
            .map_err(|e| format!("Failed to parse medical rule edits: {e}")),
        None => Ok(Vec::new()),
    }
}

pub fn save(app: &AppHandle, edits: &[RuleEdit]) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(edits)
        .map_err(|e| format!("Failed to serialize medical rule edits: {e}"))?;
    store.set(EDITS_KEY, value);
    Ok(())
}
