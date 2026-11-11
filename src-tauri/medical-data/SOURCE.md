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

Set up with Claude Code.
