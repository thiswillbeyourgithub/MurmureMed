//! French medical post-processing (MurmureMed fork only, not for upstream).

mod drug_rules;

use drug_rules::DrugRules;

/// Fixes drug names and medical terms that speech recognition mishears.
/// Returns the text unchanged if the bundled rules failed to load.
pub fn apply_medical_fixes(text: String) -> String {
    match DrugRules::bundled() {
        Some(rules) => rules.apply(&text),
        None => text,
    }
}
