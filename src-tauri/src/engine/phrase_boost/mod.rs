//! Phrase boosting ported from ParakeetWeb (written with Claude Code): a list
//! of phrases (`list`) is expanded into spellings, tokenized, and stored in a
//! trie (`trie`) that the decoder queries to nudge the scores of tokens
//! continuing a listed phrase.

pub mod list;
pub mod trie;

use tokenizers::Tokenizer;

use super::helpers::tokenize_word_to_ids;
use list::BoostPhrase;
pub use trie::{BoostingTrie, EncodedPhrase, NodeId};

/// Tokenize every spelling. Spellings that tokenize to nothing or contain an
/// unknown token (a character absent from the vocabulary, e.g. CJK) can never
/// be emitted by the decoder: they are returned apart, to be reported.
pub fn encode_phrases(
    phrases: &[BoostPhrase],
    tokenizer: &Tokenizer,
) -> (Vec<EncodedPhrase>, Vec<String>) {
    let mut encoded = Vec::with_capacity(phrases.len());
    let mut skipped = Vec::new();
    for p in phrases {
        match tokenize_word_to_ids(tokenizer, &p.phrase) {
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

#[cfg(test)]
mod tests {
    use super::list::{expand_augmentations, parse_boost_list};
    use super::*;

    fn bundled_tokenizer() -> Option<Tokenizer> {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/tokenizer.json");
        Tokenizer::from_file(path).ok()
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
    /// `MURMUREMED_BOOST_LIST=path/to/list.txt cargo test --release --lib phrase_boost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn build_stats_of_a_real_list() {
        let path = std::env::var("MURMUREMED_BOOST_LIST").expect("MURMUREMED_BOOST_LIST");
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
