//! Drug-name and medical-term fix rules (MurmureMed fork).
//!
//! Port of ParakeetWeb's `app/ui/src/lib/drugRules.js`: ~40,000 regexes that
//! fix how French speech recognition mishears drug names and medical terms
//! ("l'ananas de l'umab" -> "lanadelumab", "ostéophite" -> "ostéophyte").
//! The rules are the UltiMed model repo's `regex-fixes/*.jsonl` files, copied
//! byte for byte into `src-tauri/medical-data/regex-fixes/` (see
//! `medical-data/SOURCE.md`) and applied in order: manual, drug, term.
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
//! its anchor (the longest accent-folded word of a misspelling it fixes), so a
//! typical sentence compiles and runs ~100 rules instead of 40,000. The result
//! is the same as running every rule in file order on the whole text.
//!
//! User edits ([`RuleEdit`]) are an overlay on the bundled rules: a rule can be
//! disabled or get a new core and replacement, and new rules run before all
//! bundled ones. Bundled regexes stay compiled across edits.
//!
//! Written with Claude Code.

use log::warn;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

/// The bundled rule files, in the order they are applied.
const BUNDLED_SOURCES: [&str; 3] = [
    include_str!("../../medical-data/regex-fixes/manual_fix_rules.jsonl"),
    include_str!("../../medical-data/regex-fixes/drug_fix_rules.jsonl"),
    include_str!("../../medical-data/regex-fixes/term_fix_rules.jsonl"),
];

const WORD_CLASS: &str = r"[0-9A-Za-z_\u00C0-\u00D6\u00D8-\u00F6\u00F8-\u024F";
/// Consuming equivalent of SUFFIX: a non-word, non-hyphen char, or the end.
const SUFFIX_REGEX: &str = r"(?:[^0-9A-Za-z_\x{C0}-\x{D6}\x{D8}-\x{F6}\x{F8}-\x{24F}\-]|\z)";

/// One line of a `regex-fixes/*.jsonl` file (other fields are ignored).
#[derive(Deserialize)]
struct SourceRule {
    pattern: String,
    replacement: String,
    #[serde(default)]
    variants: Vec<String>,
    #[serde(default)]
    variant: String,
}

/// A user change to the medical rules, stored in `medical_rules.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleEdit {
    /// Full pattern of the bundled rule this edit replaces, or `None` for a
    /// new rule (new rules run before every bundled one).
    pub original: Option<String>,
    /// Pattern core: what to match, without the word boundaries, which are
    /// always added. Case-insensitive.
    pub core: String,
    pub replacement: String,
    pub enabled: bool,
}

struct Rule {
    /// Lowercase titles ("docteur ") the match must not directly follow.
    exclusions: Vec<Vec<char>>,
    core: String,
    replacement: String,
    regex: OnceLock<Option<Regex>>,
}

/// The bundled rules, parsed once.
pub struct Bundled {
    /// Full patterns, the key of [`RuleEdit::original`]; `None` for blank lines.
    patterns: Vec<String>,
    rules: Vec<Option<Rule>>,
    anchors: HashMap<String, Vec<u32>>,
    always: Vec<u32>,
}

/// Bundled rules plus user edits: what [`DrugRules::apply`] runs. Rule ids
/// are `0..added.len()` for new rules, then `added.len() + i` for bundled
/// rule `i`.
pub struct DrugRules {
    base: Arc<Bundled>,
    added: Vec<Rule>,
    edited: HashMap<u32, Option<Rule>>,
    always: Vec<u32>,
}

/// A bundled rule as the settings UI shows it.
#[derive(Debug, Clone, Serialize)]
pub struct RuleView {
    pub original: String,
    pub core: String,
    pub replacement: String,
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

/// The words a rule is indexed under, or `None` when one of its misspellings
/// cannot be anchored on a word (the rule then runs on every text). Port of
/// the anchor step of ParakeetWeb's `compileDrugRuleSource`.
fn anchors_of(rule: &SourceRule) -> Option<HashSet<String>> {
    let single = [rule.variant.clone()];
    let variants = if rule.variants.is_empty() {
        &single[..]
    } else {
        &rule.variants[..]
    };
    let mut anchors = HashSet::new();
    for variant in variants {
        let folded = fold(variant);
        let words: Vec<&str> = folded
            .trim()
            .split(|c: char| c.is_whitespace() || is_apostrophe(c) || c == '-')
            .collect();
        if !words
            .iter()
            .all(|w| !w.is_empty() && w.chars().all(is_word_char))
        {
            return None;
        }
        let longest = words.iter().fold("", |a, w| {
            if w.chars().count() > a.chars().count() {
                w
            } else {
                a
            }
        });
        anchors.insert(longest.to_string());
    }
    Some(anchors)
}

/// Control and bidi-override chars: the output can land in the clipboard, so
/// a tampered rule file must not be able to inject them.
fn is_unsafe_replacement(replacement: &str) -> bool {
    replacement.chars().any(|c| {
        matches!(c, '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
    })
}

fn has_lookaround(core: &str) -> bool {
    ["(?<", "(?=", "(?!"].iter().any(|l| core.contains(l))
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
    if has_lookaround(core) {
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

impl Rule {
    fn new(exclusions: Vec<Vec<char>>, core: &str, replacement: &str) -> Option<Self> {
        if replacement.is_empty() || is_unsafe_replacement(replacement) {
            return None;
        }
        Some(Self {
            exclusions,
            core: core.to_string(),
            replacement: replacement.to_string(),
            regex: OnceLock::new(),
        })
    }

    fn from_pattern(pattern: &str, replacement: &str) -> Option<Self> {
        let (exclusions, core) = parse_pattern(pattern)?;
        Self::new(exclusions, core, replacement)
    }

    fn compile(core: &str) -> Result<Regex, regex::Error> {
        Regex::new(&format!("((?i:{core})){SUFFIX_REGEX}"))
    }

    fn regex(&self) -> Option<&Regex> {
        self.regex
            .get_or_init(|| match Self::compile(&self.core) {
                Ok(re) => Some(re),
                Err(e) => {
                    warn!("Drug rule {:?} does not compile: {e}", self.replacement);
                    None
                }
            })
            .as_ref()
    }

    /// This rule on every match in `text`, or `None` when it does not fire.
    fn apply(&self, text: &str) -> Option<String> {
        let re = self.regex()?;
        let mut out = String::new();
        let mut copied = 0;
        let mut search = 0;
        let mut fired = false;
        while search <= text.len() {
            let Some(m) = re.captures_at(text, search).and_then(|c| c.get(1)) else {
                break;
            };
            if !boundary_ok(text, m.start(), &self.exclusions) {
                // Lookbehinds only depend on the start: retry one char later.
                search = m.start() + text[m.start()..].chars().next().map_or(1, char::len_utf8);
                continue;
            }
            fired = true;
            out.push_str(&text[copied..m.start()]);
            out.push_str(&cased_replacement(m.as_str(), &self.replacement));
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
}

/// Checks a user edit before it is saved, so the UI can show why it is refused.
pub fn validate_edit(edit: &RuleEdit) -> Result<(), String> {
    if edit.core.trim().is_empty() {
        return Err("The pattern is empty".to_string());
    }
    if edit.replacement.is_empty() || is_unsafe_replacement(&edit.replacement) {
        return Err("The replacement is empty or contains control characters".to_string());
    }
    if has_lookaround(&edit.core) {
        return Err("Lookarounds are not supported".to_string());
    }
    Rule::compile(&edit.core)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

impl Bundled {
    /// Parses rule files given as JSON lines, in the order they apply.
    /// Unusable lines become `None` rules, so one bad rule never takes the
    /// others down with it.
    pub fn from_jsonl(sources: &[&str]) -> Self {
        let mut patterns = Vec::new();
        let mut rules = Vec::new();
        let mut anchors: HashMap<String, Vec<u32>> = HashMap::new();
        let mut always = Vec::new();
        let lines = sources
            .iter()
            .flat_map(|s| s.lines())
            .filter(|l| !l.trim().is_empty());
        for (id, line) in lines.enumerate() {
            let id = id as u32;
            let Ok(source) = serde_json::from_str::<SourceRule>(line) else {
                patterns.push(String::new());
                rules.push(None);
                continue;
            };
            match anchors_of(&source) {
                Some(words) => {
                    for w in words {
                        anchors.entry(w).or_default().push(id);
                    }
                }
                None => always.push(id),
            }
            rules.push(Rule::from_pattern(&source.pattern, &source.replacement));
            patterns.push(source.pattern);
        }
        let skipped = rules.iter().filter(|r| r.is_none()).count();
        if skipped > 0 {
            warn!("Drug rules: skipped {skipped} unusable rules");
        }
        Self {
            patterns,
            rules,
            anchors,
            always,
        }
    }

    /// The rules shipped with the app, parsed on first use.
    pub fn shared() -> Arc<Bundled> {
        static BUNDLED: OnceLock<Arc<Bundled>> = OnceLock::new();
        BUNDLED
            .get_or_init(|| Arc::new(Bundled::from_jsonl(&BUNDLED_SOURCES)))
            .clone()
    }

    fn id_of(&self, pattern: &str) -> Option<u32> {
        self.patterns
            .iter()
            .position(|p| p == pattern)
            .map(|i| i as u32)
    }

    /// Bundled rules whose replacement contains `query` (accents and case
    /// ignored), or which would rewrite `query` itself: typing either the
    /// right word or the misheard one finds the rule.
    pub fn search(&self, query: &str, limit: usize) -> Vec<RuleView> {
        let needle = fold(query.trim());
        if needle.is_empty() {
            return Vec::new();
        }
        let mut ids: BTreeSet<u32> = tokens(query)
            .iter()
            .filter_map(|t| self.anchors.get(t))
            .flatten()
            .chain(&self.always)
            .copied()
            .filter(|id| {
                self.rules[*id as usize]
                    .as_ref()
                    .and_then(|r| r.apply(query))
                    .is_some_and(|out| out != query)
            })
            .collect();
        ids.extend(
            self.rules
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    r.as_ref()
                        .is_some_and(|r| fold(&r.replacement).contains(&needle))
                })
                .map(|(i, _)| i as u32)
                .take(limit),
        );
        ids.into_iter()
            .take(limit)
            .filter_map(|id| {
                let rule = self.rules[id as usize].as_ref()?;
                Some(RuleView {
                    original: self.patterns[id as usize].clone(),
                    core: rule.core.clone(),
                    replacement: rule.replacement.clone(),
                })
            })
            .collect()
    }
}

impl DrugRules {
    /// The bundled rules with `edits` applied. Invalid edits are skipped (the
    /// settings UI validates them before saving).
    pub fn new(base: Arc<Bundled>, edits: &[RuleEdit]) -> Self {
        let mut added = Vec::new();
        let mut edited = HashMap::new();
        let mut changed_core = Vec::new();
        for edit in edits {
            if edit.enabled && validate_edit(edit).is_err() {
                warn!("Skipping an invalid medical rule edit");
                continue;
            }
            let rule = edit
                .enabled
                .then_some(())
                .and_then(|_| match &edit.original {
                    None => Rule::new(Vec::new(), &edit.core, &edit.replacement),
                    Some(original) => {
                        let (exclusions, _) = parse_pattern(original)?;
                        Rule::new(exclusions, &edit.core, &edit.replacement)
                    }
                });
            match &edit.original {
                None => added.extend(rule),
                Some(original) => match base.id_of(original) {
                    Some(id) => {
                        let same_core = base.rules[id as usize]
                            .as_ref()
                            .is_some_and(|r| r.core == edit.core);
                        if !same_core {
                            changed_core.push(id);
                        }
                        edited.insert(id, rule);
                    }
                    None => warn!("Medical rule edit no longer matches a bundled rule"),
                },
            }
        }
        let offset = added.len() as u32;
        let mut always: Vec<u32> = (0..offset).collect();
        always.extend(
            base.always
                .iter()
                .chain(&changed_core)
                .map(|id| id + offset),
        );
        Self {
            base,
            added,
            edited,
            always,
        }
    }

    fn rule(&self, id: u32) -> Option<&Rule> {
        let offset = self.added.len() as u32;
        if id < offset {
            return self.added.get(id as usize);
        }
        let base_id = id - offset;
        match self.edited.get(&base_id) {
            Some(edit) => edit.as_ref(),
            None => self.base.rules[base_id as usize].as_ref(),
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.added.len() + self.base.rules.len()
    }

    fn candidates(&self, text: &str, after: Option<u32>) -> Vec<u32> {
        let offset = self.added.len() as u32;
        tokens(text)
            .iter()
            .filter_map(|t| self.base.anchors.get(t))
            .flatten()
            .map(|id| id + offset)
            .filter(|id| after.is_none_or(|a| *id > a))
            .collect()
    }

    /// Applies the rules through the anchor index. When a rule changes the
    /// text, the new words are looked up again, but only rules after it are
    /// queued, exactly what an in-order pass over all rules would still run.
    pub fn apply(&self, text: &str) -> String {
        let mut text = text.to_string();
        let mut todo: BTreeSet<u32> = self.candidates(&text, None).into_iter().collect();
        todo.extend(&self.always);
        while let Some(id) = todo.pop_first() {
            let Some(next) = self.rule(id).and_then(|r| r.apply(&text)) else {
                continue;
            };
            if next != text {
                text = next;
                todo.extend(self.candidates(&text, Some(id)));
            }
        }
        text
    }

    /// Reference semantics: every rule in order. Only for tests.
    #[cfg(test)]
    fn apply_naive(&self, text: &str) -> String {
        let mut text = text.to_string();
        for id in 0..self.len() as u32 {
            if let Some(next) = self.rule(id).and_then(|r| r.apply(&text)) {
                text = next;
            }
        }
        text
    }
}

#[cfg(test)]
#[path = "drug_rules_tests.rs"]
mod tests;
