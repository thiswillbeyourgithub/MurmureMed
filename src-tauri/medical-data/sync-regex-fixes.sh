#!/bin/sh
# Refresh the bundled UltiMed fix rules from a local clone of the model repo
# (https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-onnx).
#
#   ./sync-regex-fixes.sh [path/to/parakeet-tdt-0.6b-v3-ultra-UltiMed-onnx]
#
# Copies regex-fixes/*.jsonl byte for byte and records each file's commit and
# sha256 in regex-fixes/PROVENANCE, then reminds you to run the tests.
# Written with Claude Code.
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
repo="${1:-$here/../../../ASR_parakeet_finetuning_ultimed_nemo/parakeet-tdt-0.6b-v3-ultra-UltiMed-onnx}"
src="$repo/regex-fixes"
dst="$here/regex-fixes"
# Applied in this order: hand-written fixes win over learned ones.
files="manual_fix_rules.jsonl drug_fix_rules.jsonl term_fix_rules.jsonl"

[ -d "$src" ] || { echo "ERROR: $src not found" >&2; exit 1; }
mkdir -p "$dst"
: > "$dst/PROVENANCE"
for f in $files; do
    cp "$src/$f" "$dst/$f"
    commit="$(git -C "$repo" log -1 --format=%H -- "regex-fixes/$f" 2>/dev/null || echo unknown)"
    if [ -n "$(git -C "$repo" status --porcelain -- "regex-fixes/$f" 2>/dev/null)" ]; then
        commit="$commit (plus uncommitted changes)"
    fi
    sha="$(sha256sum "$dst/$f" | cut -d' ' -f1)"
    echo "$f $sha $commit" >> "$dst/PROVENANCE"
done
cat "$dst/PROVENANCE"
echo "Now run: cd src-tauri && cargo test --lib medical"
