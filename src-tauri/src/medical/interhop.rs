//! InterHop's French medical dictation rules (units, constants, dictated
//! punctuation, exam templates...), seeded as the user's formatting rules on a
//! fresh install. Unlike the drug fixes they are ordinary user rules from then
//! on: the user edits, disables or deletes them in the formatting rules UI.
//! Bundled by `medical-data/sync-interhop-rules.py`, see `medical-data/SOURCE.md`.

use crate::formatting_rules::types::{FormattingRule, MatchMode};
use log::warn;
use serde::Deserialize;

const BUNDLED: &str = include_str!("../../medical-data/interhop/rules.json");

#[derive(Deserialize)]
struct InterhopRule {
    id: String,
    trigger: String,
    replacement: String,
    #[cfg_attr(not(test), allow(dead_code))]
    example_pre: String,
    #[cfg_attr(not(test), allow(dead_code))]
    example_post: String,
}

fn bundled() -> Vec<InterhopRule> {
    serde_json::from_str(BUNDLED).unwrap_or_else(|e| {
        warn!("Could not parse the bundled InterHop rules: {e}");
        Vec::new()
    })
}

/// The formatting rules of a fresh install.
pub fn default_rules() -> Vec<FormattingRule> {
    bundled()
        .into_iter()
        .map(|rule| FormattingRule {
            id: rule.id,
            trigger: rule.trigger,
            replacement: rule.replacement,
            enabled: true,
            match_mode: MatchMode::Regex,
            name: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatting_rules::types::BuiltInOptions;
    use crate::formatting_rules::{apply_formatting, FormattingSettings};

    /// Only the given user rules, every built-in option off.
    fn rules_only(rules: Vec<FormattingRule>) -> FormattingSettings {
        FormattingSettings {
            built_in: BuiltInOptions {
                short_text_correction: 0,
                trailing_space: false,
                medical_fixes: false,
                ..BuiltInOptions::default()
            },
            rules,
        }
    }

    #[test]
    fn fresh_install_gets_the_interhop_rules() {
        let rules = FormattingSettings::default().rules;
        assert_eq!(rules.len(), bundled().len());
        assert!(rules.len() > 100);
        assert!(rules
            .iter()
            .all(|r| r.enabled && r.match_mode == MatchMode::Regex));
    }

    /// Every example whose output differs from the expected text, when run
    /// through the settings that `settings_for` builds from the example's rule.
    fn mismatches(settings_for: impl Fn(FormattingRule) -> FormattingSettings) -> String {
        default_rules()
            .into_iter()
            .zip(bundled())
            .filter_map(|(rule, source)| {
                let got = apply_formatting(source.example_pre.clone(), &settings_for(rule));
                (got != source.example_post).then(|| {
                    format!(
                        "{}: {:?} -> {got:?}, expected {:?}",
                        source.id, source.example_pre, source.example_post
                    )
                })
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Each rule alone turns its example into the expected text, with
    /// Murmure's regex engine (InterHop checks them with Python's `re`).
    #[test]
    fn each_rule_fixes_its_example() {
        let failures = mismatches(|rule| rules_only(vec![rule]));
        assert!(failures.is_empty(), "{failures}");
    }

    /// All the rules together still give each example's expected text: no rule
    /// undoes another one's work.
    #[test]
    fn rules_do_not_interfere() {
        let all = rules_only(default_rules());
        let failures = mismatches(|_| all.clone());
        assert!(failures.is_empty(), "{failures}");
    }
}
