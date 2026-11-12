//! French medical post-processing (Plumed fork only, not for upstream).
//!
//! The bundled drug and medical-term fix rules, plus the user's edits to them,
//! kept apart from the user's own formatting rules: those run after, and an
//! import or a reset of either side leaves the other alone.

mod drug_rules;
mod interhop;
pub mod phrase_list;
mod store;

pub use drug_rules::{validate_edit, RuleEdit, RuleView};
use drug_rules::{Bundled, DrugRules};
pub use interhop::default_rules as default_formatting_rules;
use log::warn;
use std::sync::{Arc, RwLock};
use tauri::AppHandle;

/// The rules [`apply_medical_fixes`] runs: bundled rules plus saved edits.
static ACTIVE: RwLock<Option<Arc<DrugRules>>> = RwLock::new(None);

fn activate(edits: &[RuleEdit]) {
    let rules = Arc::new(DrugRules::new(Bundled::shared(), edits));
    match ACTIVE.write() {
        Ok(mut active) => *active = Some(rules),
        Err(e) => warn!("Medical rules lock poisoned: {e}"),
    }
}

fn active() -> Arc<DrugRules> {
    if let Some(rules) = ACTIVE.read().ok().and_then(|a| a.clone()) {
        return rules;
    }
    activate(&[]);
    ACTIVE
        .read()
        .ok()
        .and_then(|a| a.clone())
        .unwrap_or_else(|| Arc::new(DrugRules::new(Bundled::shared(), &[])))
}

/// Parses the bundled rules and loads the saved edits. Call once at startup,
/// off the main thread: parsing ~40,000 rules takes a fraction of a second.
pub fn init(app: &AppHandle) {
    match store::load(app) {
        Ok(edits) => activate(&edits),
        Err(e) => {
            warn!("Could not load medical rule edits, using the bundled rules: {e}");
            activate(&[]);
        }
    }
}

/// Fixes drug names and medical terms that speech recognition mishears.
pub fn apply_medical_fixes(text: String) -> String {
    active().apply(&text)
}

pub fn get_edits(app: &AppHandle) -> Result<Vec<RuleEdit>, String> {
    store::load(app)
}

/// Validates, saves and activates the edits. Nothing is saved if one is invalid.
pub fn set_edits(app: &AppHandle, edits: Vec<RuleEdit>) -> Result<(), String> {
    for edit in &edits {
        validate_edit(edit).map_err(|e| format!("{}: {e}", edit.replacement))?;
    }
    store::save(app, &edits)?;
    activate(&edits);
    Ok(())
}

/// Bundled rules matching `query`, see [`Bundled::search`].
pub fn search(query: &str, limit: usize) -> Vec<RuleView> {
    Bundled::shared().search(query, limit)
}
