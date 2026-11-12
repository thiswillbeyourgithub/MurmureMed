#!/bin/sh
# Refresh the bundled French medical phrase boosting list from a local clone of
# ParakeetWeb (https://github.com/thiswillbeyourgithub/parakeet_web).
#
#   ./sync-phrase-list.sh [path/to/parakeet_web]
#
# Copies phrase_boosting/french_medical.txt with its first line (the defaults
# line, `*:1.5::fhpn`) replaced by Plumed's tuning, and records the source
# commit and sha256 in phrase-boosting/PROVENANCE.
# Written with Claude Code.
set -eu

here="$(cd "$(dirname "$0")" && pwd)"
repo="${1:-$here/../../../parakeet_web}"
src="$repo/phrase_boosting/french_medical.txt"
dst="$here/phrase-boosting/french_medical.txt"
# Benchmarked on UltiMed validation clips (see AGENTS.FORK.md): full bonus from
# the first token (commitment 0) and a looser min-p gate (0.01) than PW's 0.1.
header='#!commitment 0
*:1.5:0.01:fhpn'

[ -f "$src" ] || { echo "ERROR: $src not found" >&2; exit 1; }
case "$(head -n 1 "$src")" in
    '*:'*) ;;
    *) echo "ERROR: $src does not start with a defaults line" >&2; exit 1 ;;
esac
mkdir -p "$(dirname "$dst")"
{ echo "$header"; tail -n +2 "$src"; } > "$dst"
commit="$(git -C "$repo" log -1 --format=%H -- phrase_boosting/french_medical.txt 2>/dev/null || echo unknown)"
if [ -n "$(git -C "$repo" status --porcelain -- phrase_boosting/french_medical.txt 2>/dev/null)" ]; then
    commit="$commit (plus uncommitted changes)"
fi
echo "french_medical.txt $(sha256sum "$src" | cut -d' ' -f1) $commit" > "$here/phrase-boosting/PROVENANCE"
cat "$here/phrase-boosting/PROVENANCE"
echo "Now run: cd src-tauri && cargo test --lib phrase_list"
