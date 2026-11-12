//! Phrase list entry points on upstream's engine types. Kept in this module
//! (written with Claude Code) so the upstream engine files only gain the
//! `phrase_trie` field and the decode hook.
//!
//! Hybrid boosting, chosen from a benchmark on 200 UltiMed validation clips
//! (see AGENTS.FORK.md): upstream's dictionary boost is strong and rank gated,
//! right for a few user words but raising the WER with a ~77k phrase list,
//! where this gentler, min-p gated trie lowers it.

use super::build_trie;
use crate::engine::types::{ParakeetEngine, ParakeetModel};

impl ParakeetModel {
    /// Boost the phrases of a ParakeetWeb list, on top of the dictionary. An
    /// empty list turns phrase list boosting off.
    pub fn set_boost_list(&mut self, raw: &str) {
        self.phrase_trie = match &self.tokenizer {
            Some(tokenizer) => build_trie(raw, tokenizer),
            None => {
                log::warn!("Phrase list ignored: no tokenizer loaded");
                None
            }
        };
    }
}

impl ParakeetEngine {
    pub fn set_boost_list(&mut self, raw: &str) {
        if let Some(model) = self.model.as_mut() {
            model.set_boost_list(raw);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::pipeline::pad_with_silence;
    use crate::engine::transcription_engine::TranscriptionEngine;
    use std::path::Path;

    /// Decoding benchmark harness: transcribes every 16 kHz mono wav listed
    /// (one path per line) in MURMUREMED_BENCH_WAVS and prints one
    /// `HYP<TAB>path<TAB>text` line each. Optional boosting, combinable:
    /// MURMUREMED_BOOST_WORDS_FILE (dictionary words, one per line) and
    /// MURMUREMED_BOOST_LIST (a ParakeetWeb phrase list). Clips are padded
    /// with silence like the app does: unpadded, the model drifts into
    /// English (WER 50% instead of 18% on French medical clips).
    /// `MURMUREMED_MODEL_DIR=... MURMUREMED_BENCH_WAVS=... cargo test --release
    /// --lib phrase_boost::model::tests::bench_transcribe -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bench_transcribe() {
        let env = |k: &str| std::env::var(k).ok();
        let read = |path: String| std::fs::read_to_string(path).expect("readable file");
        let tokenizer = Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/tokenizer.json");
        let model_dir = env("MURMUREMED_MODEL_DIR").expect("MURMUREMED_MODEL_DIR");
        let mut engine =
            ParakeetEngine::load_int8(Path::new(&model_dir), Some(tokenizer)).expect("model");
        if let Some(path) = env("MURMUREMED_BOOST_WORDS_FILE") {
            let words: Vec<String> = read(path).lines().map(str::to_string).collect();
            engine.set_boost_words(&words);
        }
        if let Some(path) = env("MURMUREMED_BOOST_LIST") {
            engine.set_boost_list(&read(path));
        }
        let wavs = read(env("MURMUREMED_BENCH_WAVS").expect("MURMUREMED_BENCH_WAVS"));
        let start = std::time::Instant::now();
        for wav in wavs.lines() {
            let samples: Vec<f32> = hound::WavReader::open(wav)
                .expect("wav")
                .samples::<i16>()
                .map(|s| s.expect("sample") as f32 / 32768.0)
                .collect();
            let result = engine
                .transcribe_samples(pad_with_silence(&samples, 16000), None)
                .expect("transcribe");
            println!("HYP\t{}\t{}", wav, result.text);
        }
        eprintln!("decoded in {:?}", start.elapsed());
    }
}
