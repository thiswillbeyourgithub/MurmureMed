//! Phrase list syntax and spelling augmentations, ported from ParakeetWeb's
//! `phraseBoost.js` (same syntax, same defaults, same expansions), so a list
//! written for ParakeetWeb boosts exactly the same token sequences here.
//!
//! One phrase per line: `phrase[:WEIGHT[:MINP]][:AUG]`, fields peeled from the
//! right only when they parse (`ratio 3:1` keeps weight 1 on phrase
//! `ratio 3`, `bad:abc` is the phrase `bad:abc`). Empty fields keep the
//! default (`a::0.2`). A `*:WEIGHT:MINP:AUG` line sets the defaults of the
//! lines below it. `#!prefixes l' d'` sets the elision prefixes of the `p`
//! augmentation; any other `#!` line is a comment.
//!
//! AUG flags: `f` Title Case, `a` ALL CAPS, `p` elision prefixes, `h` symbols
//! replaced by spaces, `n` French plurals, `i` all of them, `s` none.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

pub const DEFAULT_BOOST_MIN_P: f32 = 0.1;
pub const MAX_PHRASE_WEIGHT: f32 = 10.0;
const FULL_AUGMENT: &str = "faphn";
const DEFAULT_PREFIXES: [&str; 4] = ["l'", "d'", "L'", "D'"];
const DIRECTIVE_PREFIX: &str = "#!";

static ELISION_VOWEL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^[aeiouhàâäéèêëíìîïóòôöúùûüœæ]").expect("valid regex"));
static SYMBOL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^\p{L}\p{N}\s]+").expect("valid regex"));
static SPACES_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").expect("valid regex"));
static PLURALISABLE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\p{L}[\p{L}'’-]*$").expect("valid regex"));
static INVARIANT_END_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)[sxz]$").expect("valid regex"));
static AIL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)ail$").expect("valid regex"));
static AL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)al$").expect("valid regex"));
static X_PLURAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:eau|au|eu)$").expect("valid regex"));

/// French function words left alone by the `n` (plural) augmentation.
const PLURAL_INVARIANT: [&str; 24] = [
    "de", "du", "des", "d'", "le", "la", "les", "l'", "à", "au", "aux", "en", "et", "ou", "sur",
    "pour", "par", "avec", "sans", "dans", "sous", "chez", "un", "une",
];

/// One phrase of a list, after defaults and validation.
#[derive(Debug, Clone, PartialEq)]
pub struct BoostPhrase {
    pub phrase: String,
    /// Nonzero, in `[-MAX_PHRASE_WEIGHT, MAX_PHRASE_WEIGHT]`; negative penalizes.
    pub weight: f32,
    /// A token is only boosted when its probability is at least `minp` times
    /// the most likely token's, so boosting reorders near-misses only.
    pub minp: f32,
    /// Canonical flags (subset of `faphn`, in that order); `None` = list default.
    pub augment: Option<String>,
}

/// A parsed list: its phrases plus the `#!prefixes` directive, if any.
#[derive(Debug, Clone, PartialEq)]
pub struct BoostList {
    pub phrases: Vec<BoostPhrase>,
    pub prefixes: Option<Vec<String>>,
    /// One message per line whose weight or min-p was invalid and replaced.
    pub warnings: Vec<String>,
}

fn normalize_augment(tag: &str) -> Option<String> {
    let t = tag.trim().to_lowercase();
    if t.is_empty() || !t.chars().all(|c| "sifaphn".contains(c)) {
        return None;
    }
    if t.contains('s') {
        return Some(String::new());
    }
    let full = t.contains('i');
    Some(
        FULL_AUGMENT
            .chars()
            .filter(|&c| full || t.contains(c))
            .collect(),
    )
}

fn parse_number(text: &str) -> Option<f32> {
    text.parse::<f32>().ok().filter(|v| v.is_finite())
}

enum Peeled<'a> {
    Value(&'a str, f32),
    Empty(&'a str),
}

/// Split a trailing `:number` (or empty `:`) field off `text`. A colon at the
/// start (`:0.5`) or a non-numeric tail (`bad:abc`) belongs to the phrase.
fn peel_value_field(text: &str) -> Option<Peeled<'_>> {
    let colon = text.rfind(':').filter(|&i| i > 0)?;
    let head = text[..colon].trim();
    let tail = text[colon + 1..].trim();
    if tail.is_empty() {
        return Some(Peeled::Empty(head));
    }
    parse_number(tail).map(|value| Peeled::Value(head, value))
}

struct Fields {
    phrase: String,
    weight: Option<f32>,
    minp: Option<f32>,
    augment: Option<String>,
}

fn parse_fields(text: &str) -> Fields {
    let mut phrase = text.trim();
    let mut augment = None;
    if let Some(colon) = phrase.rfind(':').filter(|&i| i > 0) {
        if let Some(flags) = normalize_augment(&phrase[colon + 1..]) {
            augment = Some(flags);
            phrase = phrase[..colon].trim();
        }
    }
    let (mut weight, mut minp) = (None, None);
    if let Some(last) = peel_value_field(phrase) {
        let (last_head, last_value) = match last {
            Peeled::Value(head, v) => (head, Some(v)),
            Peeled::Empty(head) => (head, None),
        };
        match peel_value_field(last_head) {
            Some(prev) => {
                let (prev_head, prev_value) = match prev {
                    Peeled::Value(head, v) => (head, Some(v)),
                    Peeled::Empty(head) => (head, None),
                };
                phrase = prev_head;
                weight = prev_value;
                minp = last_value;
            }
            None => {
                phrase = last_head;
                weight = last_value;
            }
        }
    }
    Fields {
        phrase: phrase.to_string(),
        weight,
        minp,
        augment,
    }
}

fn is_valid_weight(w: f32) -> bool {
    w.is_finite() && w != 0.0 && (-MAX_PHRASE_WEIGHT..=MAX_PHRASE_WEIGHT).contains(&w)
}

fn is_valid_minp(p: f32) -> bool {
    p.is_finite() && p > 0.0 && p <= 1.0
}

fn is_defaults_line(trimmed: &str) -> bool {
    trimmed == "*" || trimmed.starts_with("*:")
}

/// Apply a `*:WEIGHT:MINP:AUG` line to the running defaults. Invalid or empty
/// fields leave the previous default in place.
fn apply_defaults_line(trimmed: &str, defaults: &mut Fields) {
    let Some(rest) = trimmed.strip_prefix("*:") else {
        return;
    };
    let mut segs = rest.split(':').map(str::trim);
    if let Some(w) = segs
        .next()
        .and_then(parse_number)
        .filter(|&w| is_valid_weight(w))
    {
        defaults.weight = Some(w);
    }
    if let Some(p) = segs
        .next()
        .and_then(parse_number)
        .filter(|&p| is_valid_minp(p))
    {
        defaults.minp = Some(p);
    }
    if let Some(a) = segs
        .next()
        .filter(|a| !a.is_empty())
        .and_then(normalize_augment)
    {
        defaults.augment = Some(a);
    }
}

fn parse_prefixes_directive(body: &str) -> Option<Vec<String>> {
    let split = body.find(|c: char| c.is_whitespace() || c == '=' || c == ':')?;
    if !body[..split].eq_ignore_ascii_case("prefixes") {
        return None;
    }
    let value =
        body[split..].trim_start_matches(|c: char| c.is_whitespace() || c == '=' || c == ':');
    let list: Vec<String> = value.split_whitespace().map(str::to_string).collect();
    (!list.is_empty()).then_some(list)
}

/// Parse a whole list (ParakeetWeb's `parseBoostPhrases` + `parseBoostDirectives`).
pub fn parse_boost_list(raw: &str) -> BoostList {
    let mut list = BoostList {
        phrases: Vec::new(),
        prefixes: None,
        warnings: Vec::new(),
    };
    let mut defaults = Fields {
        phrase: String::new(),
        weight: None,
        minp: None,
        augment: None,
    };
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(body) = trimmed.strip_prefix(DIRECTIVE_PREFIX) {
            if let Some(prefixes) = parse_prefixes_directive(body.trim()) {
                list.prefixes = Some(prefixes);
            }
            continue;
        }
        if is_defaults_line(trimmed) {
            apply_defaults_line(trimmed, &mut defaults);
            continue;
        }
        let f = parse_fields(trimmed);
        let mut weight = f.weight.or(defaults.weight).unwrap_or(1.0);
        let mut minp = f.minp.or(defaults.minp).unwrap_or(DEFAULT_BOOST_MIN_P);
        if !is_valid_weight(weight) {
            list.warnings.push(format!(
                "{}: weight {} out of range [-{MAX_PHRASE_WEIGHT}, {MAX_PHRASE_WEIGHT}] (nonzero), using 1",
                f.phrase, weight
            ));
            weight = 1.0;
        }
        if !is_valid_minp(minp) {
            list.warnings.push(format!(
                "{}: min-p {} invalid (number in (0, 1]), using {DEFAULT_BOOST_MIN_P}",
                f.phrase, minp
            ));
            minp = DEFAULT_BOOST_MIN_P;
        }
        list.phrases.push(BoostPhrase {
            phrase: f.phrase,
            weight,
            minp,
            augment: f.augment.or_else(|| defaults.augment.clone()),
        });
    }
    list
}

fn strip_symbols(s: &str) -> String {
    let spaced = SYMBOL_RE.replace_all(s, " ");
    SPACES_RE.replace_all(&spaced, " ").trim().to_string()
}

fn pluralize_word(word: &str, regular: bool) -> String {
    let unchanged = word.chars().count() < 2
        || PLURAL_INVARIANT.contains(&word.to_lowercase().as_str())
        || !PLURALISABLE_RE.is_match(word)
        || word == word.to_uppercase()
        || INVARIANT_END_RE.is_match(word);
    if unchanged {
        return word.to_string();
    }
    if regular {
        return format!("{word}s");
    }
    // The suffixes below are ASCII, so these byte offsets are char boundaries.
    if AIL_RE.is_match(word) {
        return format!("{}aux", &word[..word.len() - 3]);
    }
    if AL_RE.is_match(word) {
        return format!("{}aux", &word[..word.len() - 2]);
    }
    if X_PLURAL_RE.is_match(word) {
        return format!("{word}x");
    }
    format!("{word}s")
}

/// Plural readings of a phrase: the irregular one (`journaux`) then the
/// regular one (`journals`), each only if it differs from the phrase.
fn plural_variants(phrase: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for regular in [false, true] {
        let v = phrase
            .split(' ')
            .map(|w| pluralize_word(w, regular))
            .collect::<Vec<_>>()
            .join(" ");
        if v != phrase && !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

fn prefix_applies(prefix: &str, form: &str) -> bool {
    match prefix.chars().last() {
        Some('\'') | Some('’') => ELISION_VOWEL_RE.is_match(form),
        _ => true,
    }
}

fn title_case(phrase: &str) -> String {
    phrase
        .split(' ')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every spelling a phrase expands to under `flags`, phrase first, deduplicated.
pub fn augment_variants(phrase: &str, flags: &str, prefixes: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let push = |out: &mut Vec<String>, v: String| {
        if !v.is_empty() && !out.contains(&v) {
            out.push(v);
        }
    };
    let expand = |out: &mut Vec<String>, base: &str, use_prefixes: bool| {
        let start = out.len();
        push(out, base.to_string());
        if flags.contains('f') {
            push(out, title_case(base));
        }
        if flags.contains('a') {
            push(out, base.to_uppercase());
        }
        if flags.contains('h') {
            let stripped: Vec<String> = out[start..].iter().map(|f| strip_symbols(f)).collect();
            for form in stripped {
                push(out, form);
            }
        }
        if use_prefixes && flags.contains('p') {
            let prefixed: Vec<String> = out[start..]
                .iter()
                .flat_map(|form| {
                    prefixes
                        .iter()
                        .filter(|pre| prefix_applies(pre, form))
                        .map(move |pre| format!("{pre}{form}"))
                })
                .collect();
            for form in prefixed {
                push(out, form);
            }
        }
    };
    if phrase.is_empty() {
        return out;
    }
    expand(&mut out, phrase, true);
    if flags.contains('n') {
        for plural in plural_variants(phrase) {
            expand(&mut out, &plural, false);
        }
    }
    out
}

/// Expand every phrase of a list into its spellings (its own flags, else
/// `default_augment`). When two entries yield the same spelling, the one with
/// the larger |weight| wins.
pub fn expand_augmentations(list: &BoostList, default_augment: &str) -> Vec<BoostPhrase> {
    let prefixes: Vec<String> = match &list.prefixes {
        Some(p) => p.clone(),
        None => DEFAULT_PREFIXES.iter().map(|p| p.to_string()).collect(),
    };
    let mut order: Vec<String> = Vec::new();
    let mut by_phrase: HashMap<String, BoostPhrase> = HashMap::new();
    for entry in list.phrases.iter().filter(|e| !e.phrase.is_empty()) {
        let flags = entry.augment.as_deref().unwrap_or(default_augment);
        let spellings = match flags.is_empty() {
            true => vec![entry.phrase.clone()],
            false => augment_variants(&entry.phrase, flags, &prefixes),
        };
        for spelling in spellings {
            match by_phrase.get(&spelling) {
                Some(prev) if prev.weight.abs() >= entry.weight.abs() => {}
                Some(_) => {
                    by_phrase.insert(
                        spelling.clone(),
                        BoostPhrase {
                            phrase: spelling,
                            ..entry.clone()
                        },
                    );
                }
                None => {
                    order.push(spelling.clone());
                    by_phrase.insert(
                        spelling.clone(),
                        BoostPhrase {
                            phrase: spelling,
                            ..entry.clone()
                        },
                    );
                }
            }
        }
    }
    order
        .into_iter()
        .filter_map(|p| by_phrase.remove(&p))
        .collect()
}

#[cfg(test)]
mod tests {
    // Cases ported from ParakeetWeb's test/unit/phrase-boost.test.mjs.
    use super::*;

    fn parse(raw: &str) -> Vec<BoostPhrase> {
        parse_boost_list(raw).phrases
    }

    fn summary(p: &BoostPhrase) -> (&str, f32, f32, Option<&str>) {
        (p.phrase.as_str(), p.weight, p.minp, p.augment.as_deref())
    }

    fn prefixes(list: &[&str]) -> Vec<String> {
        list.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn weights_are_peeled_from_the_right_only_when_numeric() {
        let p = parse(
            "acetaminophen\nibuprofen:2.5\n  spaced phrase  \nfoo:99\n\nratio 3:1\n:0.5\nbad:abc",
        );
        let got: Vec<(&str, f32)> = p.iter().map(|e| (e.phrase.as_str(), e.weight)).collect();
        assert_eq!(
            got,
            vec![
                ("acetaminophen", 1.0),
                ("ibuprofen", 2.5),
                ("spaced phrase", 1.0),
                ("foo", 1.0),
                ("ratio 3", 1.0),
                (":0.5", 1.0),
                ("bad:abc", 1.0),
            ]
        );
    }

    #[test]
    fn invalid_weights_fall_back_to_one_with_a_warning() {
        let list = parse_boost_list("um:-3\nover:-99\nzero:0\nfoo:99");
        let got: Vec<f32> = list.phrases.iter().map(|e| e.weight).collect();
        assert_eq!(got, vec![-3.0, 1.0, 1.0, 1.0]);
        assert_eq!(list.warnings.len(), 3);
    }

    #[test]
    fn minp_field_is_validated() {
        let p = parse("plain\nfoo:3\nbar:3:0.5\nbaz:3:0\nqux:3:2.5\nratio 3:1");
        let got: Vec<_> = p.iter().map(summary).collect();
        let d = DEFAULT_BOOST_MIN_P;
        assert_eq!(
            got,
            vec![
                ("plain", 1.0, d, None),
                ("foo", 3.0, d, None),
                ("bar", 3.0, 0.5, None),
                ("baz", 3.0, d, None),
                ("qux", 3.0, d, None),
                ("ratio 3", 1.0, d, None),
            ]
        );
    }

    #[test]
    fn augment_flags_are_parsed_and_canonicalised() {
        let p = parse("a::0.2\nb:5:0.3:i\nc:s\nd:2:i\ne:5:0.3\nf:abc:i\ng:5::fap\nh:5::p\ni:5::fa\nj:5::h\nk:5::hpf");
        let got: Vec<_> = p.iter().map(summary).collect();
        let d = DEFAULT_BOOST_MIN_P;
        assert_eq!(
            got,
            vec![
                ("a", 1.0, 0.2, None),
                ("b", 5.0, 0.3, Some("faphn")),
                ("c", 1.0, d, Some("")),
                ("d", 2.0, d, Some("faphn")),
                ("e", 5.0, 0.3, None),
                ("f:abc", 1.0, d, Some("faphn")),
                ("g", 5.0, d, Some("fap")),
                ("h", 5.0, d, Some("p")),
                ("i", 5.0, d, Some("fa")),
                ("j", 5.0, d, Some("h")),
                ("k", 5.0, d, Some("fph")),
            ]
        );
    }

    #[test]
    fn directives_are_skipped_and_prefixes_parsed() {
        let list = parse_boost_list(
            "#!prefixes l' d'\nvenlafaxine:5\n#! a comment\n#!strength 3\namlodipine",
        );
        let names: Vec<&str> = list.phrases.iter().map(|e| e.phrase.as_str()).collect();
        assert_eq!(names, vec!["venlafaxine", "amlodipine"]);
        assert_eq!(list.prefixes, Some(prefixes(&["l'", "d'"])));
        assert_eq!(parse_boost_list("#!prefixes   \nfoo").prefixes, None);
        assert_eq!(
            parse_boost_list("#!prefixes l'\n#!prefixes=d'").prefixes,
            Some(prefixes(&["d'"]))
        );
        assert_eq!(parse("#hashtag")[0].phrase, "#hashtag");
    }

    #[test]
    fn star_lines_set_running_defaults() {
        let p = parse("*:2:0.3:fa\nvenlafaxine\namlodipine:7");
        assert_eq!(summary(&p[0]), ("venlafaxine", 2.0, 0.3, Some("fa")));
        assert_eq!(summary(&p[1]), ("amlodipine", 7.0, 0.3, Some("fa")));

        let p = parse("*:2::fa\nalpha\n*:3::s\nbeta");
        assert_eq!((p[0].weight, p[0].augment.as_deref()), (2.0, Some("fa")));
        assert_eq!((p[1].weight, p[1].augment.as_deref()), (3.0, Some("")));

        let p = parse("*:::fa\nalpha");
        assert_eq!(
            summary(&p[0]),
            ("alpha", 1.0, DEFAULT_BOOST_MIN_P, Some("fa"))
        );

        let p = parse("*\n*alpha");
        assert_eq!(summary(&p[0]), ("*alpha", 1.0, DEFAULT_BOOST_MIN_P, None));

        let list = parse_boost_list("*:99\nalpha");
        assert_eq!(list.phrases[0].weight, 1.0);
        assert!(list.warnings.is_empty());

        let p = parse("*:2::fa\nalpha:9::s");
        assert_eq!((p[0].weight, p[0].augment.as_deref()), (9.0, Some("")));
    }

    #[test]
    fn casing_and_prefix_augmentations() {
        let none: Vec<String> = Vec::new();
        assert_eq!(
            augment_variants("venlafaxine", "fa", &none),
            vec!["venlafaxine", "Venlafaxine", "VENLAFAXINE"]
        );
        assert_eq!(
            augment_variants("acide folique", "f", &none),
            vec!["acide folique", "Acide Folique"]
        );
        let v = augment_variants("adulte", "p", &prefixes(&["l'", "d'"]));
        assert_eq!(v, vec!["adulte", "l'adulte", "d'adulte"]);
        // Elision prefixes only before a vowel or h.
        assert_eq!(
            augment_variants("tramadol", "p", &prefixes(&["l'"])),
            vec!["tramadol"]
        );
        assert!(
            augment_variants("hôpital", "p", &prefixes(&["l'"])).contains(&"l'hôpital".to_string())
        );
    }

    #[test]
    fn plural_augmentation() {
        let none: Vec<String> = Vec::new();
        assert_eq!(
            augment_variants("lacrymogène", "n", &none),
            vec!["lacrymogène", "lacrymogènes"]
        );
        let v = augment_variants("journal", "n", &none);
        assert!(v.contains(&"journaux".to_string()) && v.contains(&"journals".to_string()));
        let v = augment_variants("bandeau", "n", &none);
        assert!(v.contains(&"bandeaux".to_string()) && v.contains(&"bandeaus".to_string()));
        assert_eq!(augment_variants("bras", "n", &none), vec!["bras"]);
        let v = augment_variants("hernie de la paroi", "n", &none);
        assert!(v.contains(&"hernies de la parois".to_string()));
        assert!(!v.iter().any(|s| s.contains("des las")));
        assert!(augment_variants("adulte", "fn", &none).contains(&"Adultes".to_string()));
        let v = augment_variants("adulte", "pn", &prefixes(&["l'", "d'"]));
        assert!(v.contains(&"l'adulte".to_string()) && v.contains(&"adultes".to_string()));
        assert!(!v.contains(&"l'adultes".to_string()) && !v.contains(&"d'adultes".to_string()));
        for t in ["133Xe", "3TC", "5'-nucléotidase", "ADN"] {
            assert_eq!(
                augment_variants(t, "n", &none),
                vec![t],
                "{t} should take no plural"
            );
        }
        assert_eq!(
            augment_variants("adulte", "fah", &none),
            vec!["adulte", "Adulte", "ADULTE"]
        );
    }

    #[test]
    fn symbol_stripping_augmentation() {
        let none: Vec<String> = Vec::new();
        assert_eq!(
            augment_variants("alpha-methyl", "h", &none),
            vec!["alpha-methyl", "alpha methyl"]
        );
        for sym in [",", ".", "'", "\"", "-", "_", "?", "!", "`"] {
            let v = augment_variants(&format!("alpha{sym}methyl"), "h", &none);
            assert!(
                v.contains(&"alpha methyl".to_string()),
                "symbol {sym:?} not stripped"
            );
        }
        assert!(augment_variants("co-trimoxazole/IV?", "h", &none)
            .contains(&"co trimoxazole IV".to_string()));
        assert_eq!(
            augment_variants("venlafaxine", "h", &none),
            vec!["venlafaxine"]
        );
        let v = augment_variants("alpha-methyl", "ah", &none);
        assert!(v.contains(&"alpha methyl".to_string()) && v.contains(&"ALPHA METHYL".to_string()));
        let v = augment_variants("alpha-methyl", "faph", &prefixes(&["l'"]));
        for form in [
            "alpha-methyl",
            "Alpha-methyl",
            "ALPHA-METHYL",
            "alpha methyl",
            "l'alpha methyl",
        ] {
            assert!(v.contains(&form.to_string()), "missing {form}");
        }
    }

    #[test]
    fn expansion_keeps_weights_and_resolves_collisions() {
        let list = parse_boost_list("venlafaxine:5:0.3");
        assert_eq!(expand_augmentations(&list, "").len(), 1);
        let expanded = expand_augmentations(&list, "fa");
        assert_eq!(expanded.len(), 3);
        assert!(expanded.iter().all(|e| e.weight == 5.0 && e.minp == 0.3));

        assert_eq!(
            expand_augmentations(&parse_boost_list("venlafaxine::fa"), "").len(),
            3
        );
        assert_eq!(
            expand_augmentations(&parse_boost_list("venlafaxine:s"), "fa").len(),
            1
        );

        let dedup = expand_augmentations(&parse_boost_list("venlafaxine:2\nVenlafaxine:8"), "fa");
        let w = dedup
            .iter()
            .find(|e| e.phrase == "Venlafaxine")
            .map(|e| e.weight);
        assert_eq!(w, Some(8.0));
        let names: std::collections::HashSet<&str> =
            dedup.iter().map(|e| e.phrase.as_str()).collect();
        assert_eq!(names.len(), dedup.len());
    }
}
