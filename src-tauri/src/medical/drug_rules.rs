//! Drug-name and medical-term fix rules (MurmureMed fork).
//!
//! Port of ParakeetWeb's `app/ui/src/lib/drugRules.js`: ~40,000 regexes that
//! fix how French speech recognition mishears drug names and medical terms
//! ("l'ananas de l'umab" -> "lanadelumab", "ostéophite" -> "ostéophyte").
//! The data file is `src-tauri/medical-data/drug_rules.json`, kept byte for
//! byte identical to ParakeetWeb's (see `medical-data/SOURCE.md`).
//!
//! Every pattern has the same shape:
//!
//! ```text
//! (?<!(?<!W)docteur )...   optional "not right after this title" lookbehinds
//! (?<![W-])(?:(?<!['’])|(?<=(?<!W)[dlnjmtsc]['’])|(?<=qu['’]))   PREFIX
//! CORE                     no lookaround inside
//! (?![W-])                 SUFFIX
//! ```
//!
//! where `W` is the word class `[0-9A-Za-z_\u00C0-\u00D6\u00D8-\u00F6\u00F8-\u024F]`.
//! The `regex` crate has no lookarounds, so the lookbehinds are checked in
//! plain Rust at each match start, and only `CORE` plus a consuming version
//! of `SUFFIX` is compiled. This keeps the `regex` crate's linear-time
//! guarantee, and compiles about 20x faster than the full pattern with
//! fancy-regex (0.85 ms vs 18 ms per rule).
//!
//! Rules are compiled lazily: a rule only runs when one of the text's words is
//! its anchor (the anchor index is precomputed in the data file), so a typical
//! sentence compiles and runs ~100 rules instead of 40,000. The result is the
//! same as running every rule in file order on the whole text.
//!
//! Written with Claude Code.

use log::warn;
use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::OnceLock;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

const DRUG_RULES_JSON: &str = include_str!("../../medical-data/drug_rules.json");
const DRUG_RULES_FORMAT: u32 = 1;

const WORD_CLASS: &str = r"[0-9A-Za-z_\u00C0-\u00D6\u00D8-\u00F6\u00F8-\u024F";
/// Consuming equivalent of SUFFIX: a non-word, non-hyphen char, or the end.
const SUFFIX_REGEX: &str = r"(?:[^0-9A-Za-z_\x{C0}-\x{D6}\x{D8}-\x{F6}\x{F8}-\x{24F}\-]|\z)";

#[derive(Deserialize)]
struct DrugRulesFile {
    format: u32,
    rules: Vec<(String, String)>,
    anchors: HashMap<String, Vec<u32>>,
    always: Vec<u32>,
}

struct Rule {
    /// Lowercase titles ("docteur ") the match must not directly follow.
    exclusions: Vec<Vec<char>>,
    core: String,
    replacement: String,
}

pub struct DrugRules {
    rules: Vec<Option<Rule>>,
    compiled: Vec<OnceLock<Option<Regex>>>,
    anchors: HashMap<String, Vec<u32>>,
    always: Vec<u32>,
}

/// Same chars as `\u00C0-\u00D6\u00D8-\u00F6\u00F8-\u024F` plus ASCII word chars.
fn is_word_char(c: char) -> bool {
    matches!(c, '0'..='9' | 'A'..='Z' | 'a'..='z' | '_' | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{24F}')
}

fn is_apostrophe(c: char) -> bool {
    matches!(c, '\'' | '’')
}

fn eq_ignore_case(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

/// Lowercase, strip accents: "Médicament" -> "medicament".
fn fold(text: &str) -> String {
    text.to_lowercase()
        .nfkd()
        .filter(|c| !is_combining_mark(*c))
        .collect()
}

fn tokens(text: &str) -> HashSet<String> {
    fold(text)
        .split(|c: char| !is_word_char(c))
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// Control and bidi-override chars: the output can land in the clipboard, so
/// a tampered rule file must not be able to inject them.
fn is_unsafe_replacement(replacement: &str) -> bool {
    replacement.chars().any(|c| {
        matches!(c, '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
    })
}

/// Splits a pattern into its title exclusions and its core, or `None` when it
/// does not have the expected shape.
fn parse_pattern(pattern: &str) -> Option<(Vec<Vec<char>>, &str)> {
    let exclusion_start = format!("(?<!(?<!{WORD_CLASS}])");
    let prefix = format!(
        "(?<!{WORD_CLASS}-])(?:(?<!['’])|(?<=(?<!{WORD_CLASS}])[dlnjmtsc]['’])|(?<=qu['’]))"
    );
    let suffix = format!("(?!{WORD_CLASS}-])");

    let mut rest = pattern;
    let mut exclusions = Vec::new();
    while let Some(after) = rest.strip_prefix(exclusion_start.as_str()) {
        let end = after.find(')')?;
        exclusions.push(unescape_literal(&after[..end])?);
        rest = &after[end + 1..];
    }
    let core = rest
        .strip_prefix(prefix.as_str())?
        .strip_suffix(suffix.as_str())?;
    let has_lookaround = ["(?<", "(?=", "(?!"].iter().any(|l| core.contains(l));
    if has_lookaround {
        return None;
    }
    Some((exclusions, core))
}

/// `dr\. ` -> "dr. ". Rejects anything that is not a plain literal.
fn unescape_literal(escaped: &str) -> Option<Vec<char>> {
    let mut out = Vec::new();
    let mut chars = escaped.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?),
            '(' | ')' | '[' | ']' | '{' | '}' | '|' | '?' | '*' | '+' | '^' | '$' | '.' => {
                return None
            }
            _ => out.extend(c.to_lowercase()),
        }
    }
    Some(out)
}

/// PREFIX and title exclusions at byte offset `start`: what precedes the match.
fn boundary_ok(text: &str, start: usize, exclusions: &[Vec<char>]) -> bool {
    let mut before = text[..start].chars().rev();
    let c1 = before.next();
    let c2 = before.next();
    let c3 = before.next();
    let not_word = |c: Option<char>| c.is_none_or(|c| !is_word_char(c));
    let prefix_ok = match c1 {
        Some(c) if is_word_char(c) || c == '-' => false,
        Some(c) if is_apostrophe(c) => match (c2, c3) {
            (Some(b), _) if "dlnjmtsc".chars().any(|e| eq_ignore_case(b, e)) && not_word(c3) => {
                true
            }
            (Some(b), Some(a)) => eq_ignore_case(b, 'u') && eq_ignore_case(a, 'q'),
            _ => false,
        },
        _ => true,
    };
    prefix_ok
        && !exclusions
            .iter()
            .any(|title| follows_title(text, start, title))
}

/// Whether `text[..start]` ends with `title` (case-insensitive), itself not
/// preceded by a word char: JS `(?<!(?<!W)docteur )`.
fn follows_title(text: &str, start: usize, title: &[char]) -> bool {
    let mut before = text[..start].chars().rev();
    for expected in title.iter().rev() {
        match before.next() {
            Some(c) if eq_ignore_case(c, *expected) => {}
            _ => return false,
        }
    }
    before.next().is_none_or(|c| !is_word_char(c))
}

/// "Myrtazapine" -> "Mirtazapine", not "mirtazapine": a match starting with
/// a capital keeps it.
fn cased_replacement<'a>(matched: &str, replacement: &'a str) -> std::borrow::Cow<'a, str> {
    let starts_upper = matched
        .chars()
        .next()
        .is_some_and(|c| !c.to_lowercase().eq([c]));
    let mut rep = replacement.chars();
    match rep.next() {
        Some(r) if starts_upper && !r.to_uppercase().eq([r]) => {
            std::borrow::Cow::Owned(r.to_uppercase().chain(rep).collect())
        }
        _ => std::borrow::Cow::Borrowed(replacement),
    }
}

impl DrugRules {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let file: DrugRulesFile =
            serde_json::from_str(json).map_err(|e| format!("invalid drug rules file: {e}"))?;
        if file.format != DRUG_RULES_FORMAT {
            return Err(format!("unsupported drug rules format {}", file.format));
        }
        let rules: Vec<Option<Rule>> = file
            .rules
            .into_iter()
            .map(|(pattern, replacement)| {
                if replacement.is_empty() || is_unsafe_replacement(&replacement) {
                    return None;
                }
                let (exclusions, core) = parse_pattern(&pattern)?;
                Some(Rule {
                    exclusions,
                    core: core.to_string(),
                    replacement,
                })
            })
            .collect();
        let skipped = rules.iter().filter(|r| r.is_none()).count();
        if skipped > 0 {
            warn!("Drug rules: skipped {skipped} unusable rules");
        }
        let n = rules.len() as u32;
        let mut anchors = file.anchors;
        anchors.retain(|_, ids| {
            ids.retain(|id| *id < n);
            !ids.is_empty()
        });
        let always = file.always.into_iter().filter(|id| *id < n).collect();
        Ok(Self {
            compiled: (0..rules.len()).map(|_| OnceLock::new()).collect(),
            rules,
            anchors,
            always,
        })
    }

    /// The rules shipped with the app, parsed on first use.
    pub fn bundled() -> Option<&'static DrugRules> {
        static BUNDLED: OnceLock<Option<DrugRules>> = OnceLock::new();
        BUNDLED
            .get_or_init(|| match DrugRules::from_json(DRUG_RULES_JSON) {
                Ok(rules) => Some(rules),
                Err(e) => {
                    warn!("Drug rules disabled: {e}");
                    None
                }
            })
            .as_ref()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.rules.len()
    }

    fn regex(&self, id: usize) -> Option<&Regex> {
        self.compiled[id]
            .get_or_init(|| {
                let rule = self.rules[id].as_ref()?;
                match Regex::new(&format!("((?i:{})){SUFFIX_REGEX}", rule.core)) {
                    Ok(re) => Some(re),
                    Err(e) => {
                        warn!("Drug rule {id} does not compile: {e}");
                        None
                    }
                }
            })
            .as_ref()
    }

    /// Rule `id` on every match in `text`, or `None` when it does not fire.
    fn apply_rule(&self, text: &str, id: usize) -> Option<String> {
        let rule = self.rules[id].as_ref()?;
        let re = self.regex(id)?;
        let mut out = String::new();
        let mut copied = 0;
        let mut search = 0;
        let mut fired = false;
        while search <= text.len() {
            let Some(m) = re.captures_at(text, search).and_then(|c| c.get(1)) else {
                break;
            };
            if !boundary_ok(text, m.start(), &rule.exclusions) {
                // Lookbehinds only depend on the start: retry one char later.
                search = m.start() + text[m.start()..].chars().next().map_or(1, char::len_utf8);
                continue;
            }
            fired = true;
            out.push_str(&text[copied..m.start()]);
            out.push_str(&cased_replacement(m.as_str(), &rule.replacement));
            copied = m.end();
            search = if m.is_empty() {
                m.end() + text[m.end()..].chars().next().map_or(1, char::len_utf8)
            } else {
                m.end()
            };
        }
        if !fired {
            return None;
        }
        out.push_str(&text[copied..]);
        Some(out)
    }

    fn candidates(&self, text: &str, after: Option<u32>) -> impl Iterator<Item = u32> + '_ {
        tokens(text)
            .into_iter()
            .filter_map(|t| self.anchors.get(&t))
            .flatten()
            .copied()
            .filter(move |id| after.is_none_or(|a| *id > a))
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Applies the rules through the anchor index. When a rule changes the
    /// text, the new words are looked up again, but only rules after it are
    /// queued, exactly what an in-order pass over all rules would still run.
    pub fn apply(&self, text: &str) -> String {
        let mut text = text.to_string();
        let mut todo: BTreeSet<u32> = self.candidates(&text, None).collect();
        todo.extend(&self.always);
        while let Some(id) = todo.pop_first() {
            if let Some(next) = self.apply_rule(&text, id as usize) {
                if next != text {
                    text = next;
                    todo.extend(self.candidates(&text, Some(id)));
                }
            }
        }
        text
    }

    /// Reference semantics: every rule in file order. Only for tests.
    #[cfg(test)]
    fn apply_naive(&self, text: &str) -> String {
        let mut text = text.to_string();
        for id in 0..self.rules.len() {
            if let Some(next) = self.apply_rule(&text, id) {
                text = next;
            }
        }
        text
    }
}

#[cfg(test)]
#[path = "drug_rules_tests.rs"]
mod tests;
