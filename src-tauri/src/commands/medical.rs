use crate::medical::{self, RuleEdit, RuleView};
use tauri::{command, AppHandle};

/// Most search results the settings UI shows at once.
const SEARCH_LIMIT: usize = 50;

#[command]
pub fn get_medical_rule_edits(app: AppHandle) -> Result<Vec<RuleEdit>, String> {
    medical::get_edits(&app)
}

/// Saves the whole list of edits; an empty list resets to the bundled rules.
#[command]
pub fn set_medical_rule_edits(app: AppHandle, edits: Vec<RuleEdit>) -> Result<(), String> {
    medical::set_edits(&app, edits)
}

#[command]
pub fn search_medical_rules(query: String) -> Vec<RuleView> {
    medical::search(&query, SEARCH_LIMIT)
}

/// What the medical rules (with saved edits) make of `text`, to try them out.
#[command]
pub fn preview_medical_fixes(text: String) -> String {
    medical::apply_medical_fixes(text)
}
