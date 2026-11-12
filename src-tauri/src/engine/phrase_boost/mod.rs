//! Phrase boosting ported from ParakeetWeb (written with Claude Code): a list
//! of phrases (`list`) is expanded into spellings, tokenized, and stored in a
//! trie (`trie`) that the decoder queries to nudge the scores of tokens
//! continuing a listed phrase.

pub mod list;
pub mod model;
pub mod trie;

use rayon::prelude::*;
use tokenizers::Tokenizer;

use super::helpers::tokenize_word_to_ids;
use list::{expand_augmentations, parse_boost_list, BoostPhrase};
pub use trie::{BoostingTrie, EncodedPhrase, NodeId};

/// Tokenize every spelling, in parallel (a ~77k phrase list expands to ~480k
/// spellings: 7 s sequentially, 1.5 s on 12 cores). Spellings that tokenize to
/// nothing or contain an unknown token (a character absent from the
/// vocabulary, e.g. CJK) can never be emitted by the decoder: they are
/// returned apart, to be reported.
pub fn encode_phrases(
    phrases: &[BoostPhrase],
    tokenizer: &Tokenizer,
) -> (Vec<EncodedPhrase>, Vec<String>) {
    let ids: Vec<Option<Vec<i32>>> = phrases
        .par_iter()
        .map(|p| tokenize_word_to_ids(tokenizer, &p.phrase))
        .collect();
    let mut encoded = Vec::with_capacity(phrases.len());
    let mut skipped = Vec::new();
    for (p, ids) in phrases.iter().zip(ids) {
        match ids {
            Some(ids) => encoded.push(EncodedPhrase {
                ids,
                weight: p.weight,
                minp: p.minp,
            }),
            None => skipped.push(p.phrase.clone()),
        }
    }
    (encoded, skipped)
}

/// Parse a ParakeetWeb list, expand its spellings and build its trie.
/// `None` when nothing in it can be emitted by the decoder.
pub fn build_trie(raw: &str, tokenizer: &Tokenizer) -> Option<BoostingTrie> {
    let list = parse_boost_list(raw);
    for warning in &list.warnings {
        log::warn!("phrase list: {warning}");
    }
    let (encoded, skipped) = encode_phrases(&expand_augmentations(&list, ""), tokenizer);
    if !skipped.is_empty() {
        log::warn!(
            "phrase list: {} unencodable spelling(s) skipped",
            skipped.len()
        );
    }
    let trie = match list.commitment {
        Some(commitment) => BoostingTrie::with_commitment(&encoded, commitment),
        None => BoostingTrie::new(&encoded),
    };
    log::info!(
        "Phrase list boosting: {} phrase(s), {} spelling(s)",
        list.phrases.len(),
        encoded.len()
    );
    (!trie.is_empty()).then_some(trie)
}

/// Apply the trie's bonuses for the active set to `scores`, a copy of `logits`
/// that may already carry another boost (the dictionary's). When both boost a
/// token, the larger-magnitude bonus wins instead of summing, like ParakeetWeb
/// resolves two phrases proposing the same token: stacking the two systems
/// would over-boost the overlap. Blank is never boosted, so a phrase can
/// neither force nor suppress a blank.
pub fn add_phrase_bonuses(
    trie: &BoostingTrie,
    logits: &[f32],
    active: &[NodeId],
    blank_idx: i32,
    scores: &mut [f32],
) {
    for (id, bonus) in trie.boosts(logits, active) {
        let current = scores[id] - logits[id];
        if id as i32 != blank_idx && bonus.abs() > current.abs() {
            scores[id] = logits[id] + bonus;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::list::DEFAULT_BOOST_MIN_P;
    use super::*;

    fn bundled_tokenizer() -> Option<Tokenizer> {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/tokenizer.json");
        Tokenizer::from_file(path).ok()
    }

    fn bonuses_of(ids: Vec<i32>, logits: &[f32], blank_idx: i32) -> Vec<f32> {
        let trie = BoostingTrie::new(&[EncodedPhrase {
            ids,
            weight: 1.0,
            minp: DEFAULT_BOOST_MIN_P,
        }]);
        let mut scores = logits.to_vec();
        add_phrase_bonuses(&trie, logits, &trie.start(), blank_idx, &mut scores);
        scores
    }

    #[test]
    fn plausible_listed_token_gets_its_bonus() {
        // Single-token phrase, weight 1: token 1 goes from 0.6 to 1.6 and beats token 0.
        assert_eq!(
            bonuses_of(vec![1], &[1.0, 0.6, 0.0], 2),
            vec![1.0, 1.6, 0.0]
        );
    }

    #[test]
    fn token_below_min_p_is_not_boosted() {
        // ln(0.1) = -2.3: logit 0.0 is too far below 5.0.
        assert_eq!(
            bonuses_of(vec![1], &[5.0, 0.0, -1.0], 2),
            vec![5.0, 0.0, -1.0]
        );
    }

    #[test]
    fn larger_of_dictionary_and_phrase_bonus_wins() {
        let trie = BoostingTrie::new(&[EncodedPhrase {
            ids: vec![1],
            weight: 1.0,
            minp: DEFAULT_BOOST_MIN_P,
        }]);
        let logits = [1.0, 0.6, 0.0];
        let with_prior = |prior: f32| {
            let mut scores = logits.to_vec();
            scores[1] += prior;
            add_phrase_bonuses(&trie, &logits, &trie.start(), 2, &mut scores);
            scores[1]
        };
        // Phrase bonus 1.0: replaces a weaker 0.5, leaves a stronger 2.0, never sums.
        assert_eq!(with_prior(0.5), 1.6);
        assert_eq!(with_prior(2.0), 2.6);
    }

    #[test]
    fn blank_is_never_boosted() {
        assert_eq!(
            bonuses_of(vec![2], &[1.0, 0.0, 0.9], 2),
            vec![1.0, 0.0, 0.9]
        );
    }

    #[test]
    fn build_trie_applies_the_commitment_directive() {
        let Some(tokenizer) = bundled_tokenizer() else {
            return;
        };
        // syntocinon is 4 tokens: first-token bonus 1/4 by default, 1 with commitment 0.
        let first_bonus = |raw: &str| {
            let trie = build_trie(raw, &tokenizer).expect("trie");
            let mut scores = vec![0.0; 8192];
            add_phrase_bonuses(&trie, &vec![0.0; 8192], &trie.start(), -1, &mut scores);
            scores[4208]
        };
        assert_eq!(first_bonus("syntocinon:1::s"), 0.25);
        assert_eq!(first_bonus("#!commitment 0\nsyntocinon:1::s"), 1.0);
        assert!(build_trie("東京", &tokenizer).is_none());
    }

    #[test]
    fn unencodable_spellings_are_skipped() {
        let Some(tokenizer) = bundled_tokenizer() else {
            return;
        };
        let list = parse_boost_list("syntocinon\n東京");
        let (encoded, skipped) = encode_phrases(&list.phrases, &tokenizer);
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0].ids, vec![4208, 434, 4387, 289]);
        assert_eq!(skipped, vec!["東京".to_string()]);
    }

    /// Size and build time of a real list:
    /// `PLUMED_BOOST_LIST=path/to/list.txt cargo test --release --lib phrase_boost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn build_stats_of_a_real_list() {
        let path = std::env::var("PLUMED_BOOST_LIST").expect("PLUMED_BOOST_LIST");
        let tokenizer = bundled_tokenizer().expect("tokenizer");
        let raw = std::fs::read_to_string(path).expect("list");
        let start = std::time::Instant::now();
        let list = parse_boost_list(&raw);
        let expanded = expand_augmentations(&list, "");
        let parsed_at = start.elapsed();
        let (encoded, skipped) = encode_phrases(&expanded, &tokenizer);
        let encoded_at = start.elapsed();
        let trie = BoostingTrie::new(&encoded);
        println!(
            "{} phrases, {} spellings, {} skipped, {} nodes; parse+expand {:?}, encode {:?}, total {:?}",
            list.phrases.len(),
            expanded.len(),
            skipped.len(),
            trie.node_count(),
            parsed_at,
            encoded_at - parsed_at,
            start.elapsed()
        );
    }
}
