# Bundled French medical data (MurmureMed fork)

## regex-fixes/

Drug-name and medical-term fix rules (41,744 regexes), applied by `src-tauri/src/medical/drug_rules.rs` in this order: `manual_fix_rules.jsonl` (hand-written), `drug_fix_rules.jsonl`, `term_fix_rules.jsonl`.

- Byte for byte copies of the `regex-fixes/` folder of https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-onnx. The commit and SHA-256 of each file are in `regex-fixes/PROVENANCE`. Do not edit them here: users change rules from the app (their edits are stored on top, see below), and rule improvements go to the model repo.
- How the rules are built: https://github.com/thiswillbeyourgithub/UltiMed-ASR-FR-v1-scripts/tree/public/08_drug_asr_rules
- License: CC BY 4.0. Drug names come from French open data (Licence Ouverte / Etalab 2.0: OPEN_MEDIC, RETROCEDAM, BDPM); medical terms come from public French sources (terms only, no definitions). See the "Licence and attribution" section of the model repo README.

User edits made in the app (disabled, changed or added rules) are stored in `medical_rules.json` in the app data folder, separate from the user's formatting rules. Deleting them ("Reset") goes back to exactly these files.

### Refresh

```bash
# from a local clone of the model repo (default: ../../../ASR_parakeet_finetuning_ultimed_nemo/parakeet-tdt-0.6b-v3-ultra-UltiMed-onnx)
./sync-regex-fixes.sh [path/to/parakeet-tdt-0.6b-v3-ultra-UltiMed-onnx]
```

Then check that the Rust port still gives exactly ParakeetWeb's output (`app/ui/src/lib/drugRules.js`, which compiles the same JSONL files):

```bash
# in ParakeetWeb: one {"in", "out"} JSON line per text of a corpus, through compileDrugRuleSource + applyDrugRules
# then, in src-tauri/:
MURMUREMED_DRUG_PARITY=/path/to/parity.jsonl cargo test --release --lib medical -- --ignored
```

That also checks that every rule still parses and compiles.

## interhop/

InterHop's French medical dictation rules (138 regexes: units, vital signs, dictated punctuation, exam templates triggered by "ajoute la commande ..."), seeded as the user's formatting rules on a fresh install by `src-tauri/src/medical/interhop.rs`. From then on they are ordinary user rules: editable, removable, exported and imported like any other. Existing installs keep their rules; they can import InterHop's `config.murmure` with "Fusionner" (same rule ids, so nothing is duplicated, but that import also overwrites the built-in options).

- Source: https://framagit.org/interhop/murmure-regex (`regex.csv`). The commit is in `interhop/PROVENANCE`.
- `rules.json` is generated from `regex.csv` by InterHop's own converter (`convert_csv_to_murmure.py`, loaded from the clone): same parsing, same skipped rows, same ids as their `config.murmure`. Each rule keeps its example, and the Rust tests check that every example gives the expected text, alone and with all the rules together.
- License: TODO. The repository has no license file; the MurmureMed maintainer is clarifying this with InterHop.

### Refresh

```bash
# from a local clone of murmure-regex (default: ../../../parakeet_web/framagit_murmure)
python sync-interhop-rules.py [path/to/murmure-regex]
cd .. && cargo test --lib interhop
```

## phrase-boosting/

French medical phrase boosting list (~77,000 phrases: drugs, medical terms, LoRN bacteria names, dosage units), loaded into the decoder at model load by `src-tauri/src/medical/phrase_list.rs`. During decoding it nudges the model toward these phrases (ParakeetWeb's trie engine, `src-tauri/src/engine/phrase_boost/`), on top of the user dictionary.

- Copy of ParakeetWeb's `phrase_boosting/french_medical.txt` (https://github.com/thiswillbeyourgithub/parakeet_web) with only its first line, the defaults line, replaced by MurmureMed's tuning (`#!commitment 0` and min-p 0.01, chosen by the benchmark in `AGENTS.FORK.md`). The source commit and SHA-256 are in `phrase-boosting/PROVENANCE`. Do not edit it here: list improvements go to ParakeetWeb.
- License: AGPL-3.0, like ParakeetWeb and Murmure. TODO: confirm the licenses of the data the list is generated from (ultimate_french_medical_dataset sources, LPSN names).

### Refresh

```bash
# from a local clone of ParakeetWeb (default: ../../../parakeet_web)
./sync-phrase-list.sh [path/to/parakeet_web]
cd .. && cargo test --lib phrase_list
```

Set up with Claude Code.
