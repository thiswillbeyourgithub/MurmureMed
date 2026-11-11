# Bundled French medical data (MurmureMed fork)

## drug_rules.json

Drug-name and medical-term fix rules (39,961 regexes), used by `src-tauri/src/medical/drug_rules.rs`.

- Byte for byte copy of ParakeetWeb's `app/ui/public/drug-rules/drug_rules.json` (https://github.com/thiswillbeyourgithub/parakeet_web, commit `e19d721`, "Drug rules: recompile with the redux rebuild (39,961 rules)"). Do not edit it here: refresh it in ParakeetWeb (`node scripts/compile-drug-rules.mjs`), then copy it over.
- Compiled from the `regex-fixes/` rule files of https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-UltiMed-onnx. The path, commit and SHA-256 of each source file are recorded in the file itself (`sources`).
- How the rules are built: https://github.com/thiswillbeyourgithub/UltiMed-ASR-FR-v1-scripts/tree/public/08_drug_asr_rules
- License: CC BY 4.0. Drug names come from French open data (Licence Ouverte / Etalab 2.0: OPEN_MEDIC, RETROCEDAM, BDPM); medical terms come from public French sources (terms only, no definitions). See the "Licence and attribution" section of the model repo README.

After a refresh, check that the Rust port still matches ParakeetWeb exactly:

```bash
# in ParakeetWeb: one {"in", "out"} JSON line per text of a corpus, through applyDrugRules
# then, in src-tauri/:
MURMUREMED_DRUG_PARITY=/path/to/parity.jsonl cargo test --release --lib medical -- --ignored
```

That also checks that every rule still parses and compiles.

Set up with Claude Code.
