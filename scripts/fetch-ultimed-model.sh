#!/usr/bin/env bash
# Plumed fork: fetch the UltiMed fine-tuned Parakeet model (int8 ONNX) into
# resources/parakeet-tdt-0.6b-v3-int8/, the folder the app loads its model from
# (MODEL_FILENAME in src-tauri/src/model/model.rs). The folder keeps upstream's name so
# no Rust code changes, even though it now holds UltiMed instead of the stock model.
#
# Model: https://huggingface.co/Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-onnx
# (French medical fine-tune of Parakeet TDT 0.6B v3, same vocabulary and file layout).
#
# Usage:
#   scripts/fetch-ultimed-model.sh                   # download from Hugging Face (used by CI)
#   scripts/fetch-ultimed-model.sh path/to/clone     # copy from a local clone of the model repo
#
# Every file is checked against the SHA-256 below. To move to a new model revision, update
# REVISION and the hashes together.
#
# Set up with Claude Code.
set -euo pipefail

REPO="Olicorne/parakeet-tdt-0.6b-v3-ultra-UltiMed-onnx"
REVISION="ed3164a5aa5b0cd8a3e5627126109cc2483b0449"
DEST="resources/parakeet-tdt-0.6b-v3-int8"

# "<path in the model repo> <sha256>", installed flat into DEST
FILES=(
    "int8/encoder-model.int8.onnx f524b0d415dd142502fd504fb7dfe722f455edbe280131ac8fadf9eab3af23a3"
    "int8/decoder_joint-model.int8.onnx 5c621d9d3769505c764e7c2de0c7508427cc1afdba73e1174003d52d4b784468"
    "nemo128.onnx a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f"
    "vocab.txt d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d"
    "config.json 666903c76b9798caf2c210afd4f6cd60b08a8dbf9800ec8d7a3bc0d2148ac466"
)

sha256() {
    if command -v sha256sum > /dev/null; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1 # macOS
    fi
}

LOCAL_CLONE="${1:-}"
mkdir -p "$DEST"

for entry in "${FILES[@]}"; do
    path="${entry% *}"
    expected="${entry##* }"
    out="$DEST/$(basename "$path")"
    if [ -n "$LOCAL_CLONE" ]; then
        echo "Copying $LOCAL_CLONE/$path"
        cp "$LOCAL_CLONE/$path" "$out"
    else
        url="https://huggingface.co/$REPO/resolve/$REVISION/$path"
        echo "Downloading $url"
        curl --proto '=https' --fail --retry 3 -L -o "$out" "$url"
    fi
    actual="$(sha256 "$out")"
    if [ "$actual" != "$expected" ]; then
        echo "::error::SHA-256 mismatch for $path: expected $expected, got $actual" >&2
        exit 1
    fi
done

ls -al "$DEST"
