//! The bundled French medical phrase boosting list (see
//! `medical-data/SOURCE.md`), armed on the engine once, when the model loads:
//! building its trie takes a moment, unlike the per-transcription dictionary
//! sync. Written with Claude Code.

use crate::engine::ParakeetEngine;

const BUNDLED: &str = include_str!("../../medical-data/phrase-boosting/french_medical.txt");

/// Boost the bundled medical phrases on `engine`, on top of the dictionary.
pub fn arm(engine: &mut ParakeetEngine) {
    engine.set_boost_list(BUNDLED);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::phrase_boost::list::parse_boost_list;

    #[test]
    fn bundled_list_parses_with_the_benchmarked_tuning() {
        let list = parse_boost_list(BUNDLED);
        assert!(
            list.warnings.is_empty(),
            "{:?}",
            &list.warnings[..3.min(list.warnings.len())]
        );
        assert_eq!(list.commitment, Some(0.0));
        assert!(
            list.phrases.len() > 70_000,
            "{} phrases",
            list.phrases.len()
        );
        let first = &list.phrases[0];
        assert_eq!((first.weight, first.minp), (1.5, 0.01));
    }
}
