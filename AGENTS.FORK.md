# Plumed fork instructions

These instructions come from the Plumed maintainer and apply on top of [AGENTS.md](AGENTS.md) (the upstream Murmure guidelines, which still apply). Where the two conflict, this file wins.

This file was set up with Claude Code.

## 1. What this fork is

Plumed is a fork of [Murmure](https://github.com/Kieirra/murmure) specialized for **French medical dictation**. It ports the features of the medical mode of ParakeetWeb ([live](http://parakeetweb.olicorne.org/?mode=med), [code](https://github.com/thiswillbeyourgithub/parakeet_web), usually checked out next to this repo in `../parakeet_web`) into an easy to install desktop app, so dictation works in any application of the computer, not only in a browser.

Why a fork and not upstream PRs: see [Kieirra/murmure#498](https://github.com/Kieirra/murmure/issues/498). The upstream maintainer prefers that very large rule lists (~30,000 regex rules) and medical specific post-processing live in a fork. We still want to stay as close to upstream as possible and upstream every change that is generic enough.

## 2. Mandatory at the start of every session: check upstream

**Last checked upstream commit:** `8ad2363` (docs: update roadmap)
**Date of check:** 2026-11-12

At the start of every new session, before other work:

1. `git fetch upstream` (remote `upstream` = `https://github.com/Kieirra/murmure.git`; add it if missing).
2. List what is new: `git log --oneline <last checked commit>..upstream/main`.
3. Summarize these commits to the user and recommend, per commit, merge or skip, flagging anything that touches files our feature branches modify.
4. After the user approves, merge (see section 4). Default is to merge everything: staying close to upstream is the point. To skip one commit, merge anyway then `git revert` it, so the history stays mergeable.
5. Update the commit hash, its subject and the date above, and commit that change (`docs(fork): update last checked upstream commit`).

Even if there is nothing new, update the date so we know the check happened.

## 3. Minimal-diff rules (keep upstream merges painless)

- Prefer **adding new files/modules** over editing upstream ones. Hook them into upstream code with the smallest possible change (one import, one call, one config entry).
- Never reformat, reorder, rename or "clean up" upstream code we don't need to change: every touched line is a future merge conflict.
- Never edit upstream docs other than `README.md` (fully replaced by ours) and the single pointer line at the top of `AGENTS.md`. Fork documentation goes in fork-specific files (this one, or new files).
- Large data (medical regex lists, phrase boosting lists) goes in its own files, never inlined in upstream sources.
- Generic improvements (performance, bug fixes, things any Murmure user benefits from) must be written so they can be sent upstream as is: no medical-specific code mixed in.

## 4. Branching model

Branches are created only with the user's approval. **Never open a pull request upstream**: only the user does that, by hand.

- `medmain`: our default branch, what we release (named so it is never confused with `upstream/main`). Equal to upstream + fork meta files (README, this file) + every finished feature, each merged in.
- `feat/<name>`: **one feature per branch**, branched from the upstream commit we last synced to (not from `medmain`), so that `git diff upstream/main...feat/<name>` shows exactly that feature and nothing else. This makes each feature:
    - easy to review and revert (merged into `medmain` with `git merge --no-ff`, one merge commit per feature),
    - easy to benchmark in isolation (build `upstream/main` vs `feat/<name>` and compare),
    - easy for the user to propose upstream (a PR straight from the branch).
- If a feature depends on another one (example: the medical phrase list needs the new phrase boosting engine), branch it from its parent feature branch and note the dependency in the table below.
- Syncing upstream: `git merge upstream/main` into `medmain` (never rebase `medmain`, it is published). Feature branches not yet merged: rebase them onto `upstream/main` (or merge if already shared).
- `git rerere` is enabled locally (`git config rerere.enabled true`) so a conflict resolved once is resolved automatically next time.
- Recurring conflicts and how to resolve them:
    - `README.md`: always keep ours.
    - Version fields (see section 5): always keep ours.

### Feature tracking

| Feature | Branch | Depends on | Status | Upstream PR |
| --- | --- | --- | --- | --- |
| UltiMed fine-tune as default model (CI fetches the int8 export from Hugging Face, pinned and hash-checked, by `scripts/fetch-ultimed-model.sh`) | `feat/ultimed-model` | | merged (boosting keeps helping with UltiMed: drug term recall 56 to 78 of 107, same tuning) | not planned (fork only) |
| Regex engine supporting lookarounds (fancy-regex or similar) | `feat/regex-engine` | | not needed for now (the medical rules' only lookbehind, the title exclusions, is handled in code; InterHop rules have none) | |
| Compile regex once, not on every transcription | `feat/regex-compile-once` | | merged (11x to 15x faster rule application) | not planned (upstream is fine with compiling per transcription for its short lists, see #498) |
| Medical regex list (UltiMed drug and term fixes, editable in an advanced panel, separate from user rules) plus InterHop dictation rules seeded as default user rules | `feat/medical-regex` | | merged | not planned (fork only, see #498) |
| ParakeetWeb phrase boosting engine (hybrid: upstream boost for the user dictionary, ParakeetWeb trie for phrase lists, larger bonus wins where they overlap) | `feat/phrase-boosting` (from `medmain`) | | merged | |
| French medical phrase boosting list (bundled from ParakeetWeb, armed at model load, ~2 s trie build on the startup thread) | `feat/medical-phrase-list` | `feat/phrase-boosting` | merged | |
| Branding (name Plumed, purple theme, fork credits, local-only notice, own identifier and keyring entry, updater and winget; icon and toggles for extra regex and phrase boosting still to do) | `feat/branding` (from `medmain`) | | merged, in progress | |
| Remote microphone through an internet relay (or iroh) | `feat/remote-mic-relay` | | idea | |

Update this table when a branch is created, merged, or proposed upstream.

Note for `feat/medical-regex`: the `feat/regex-compile-once` cache (capped at 4096 patterns) is sized for user rules, including the 138 InterHop rules seeded by default. The ~42,000 drug and term rules do not use it: `src-tauri/src/medical/` indexes them by anchor word at startup and compiles each rule lazily, the first time a text could match it.

Note for `feat/phrase-boosting`: benchmarked on 200 UltiMed validation clips (ParakeetWeb's `benchmark_datasets/french_medical/`, padded with 0.25 s of silence like the app does; unpadded clips drift into English and give meaningless numbers). WER without boosting 12.3% (drugs) / 13.1% (dictionary). Upstream's boost fed the ~77k medical phrases raises it to 17.6% / 18.5%: its strength and rank gate suit a few user words, not a huge list. ParakeetWeb's trie with the list lowers it to 12.0% / 11.7%. Hybrid with the dictionary on top: 11.6% / 12.0%, and the most target terms recovered. The ignored test `phrase_boost::model::tests::bench_transcribe` is the harness.

## 5. Versioning

Plumed has its own version number, independent from upstream's, starting at `0.1.0` (plain semver: Cargo, `package.json` and Tauri all require exactly three numbers). The auto-updater compares it with the `version` in `latest.json`, so every release must bump it. Upstream's version is not tracked in ours: the last merged upstream commit is recorded in section 2.

The version must be identical in `package.json`, `src-tauri/Cargo.toml` (and `Cargo.lock`) and `src-tauri/tauri.conf.json`. When an upstream merge conflicts on these lines, keep ours.

Ship the NSIS `-setup.exe` as the main Windows installer: it installs without admin rights, and `latest.json` points Windows updates to it.

## 6. Releases

Identity: the app identifier is `org.olicorne.plumed` (own settings and history folder, separate from an installed Murmure; the remote LLM API key also has its own keyring entry, `plumed`). The product name is `Plumed` and the binary `plumed` (`mainBinaryName`), so Plumed installs next to Murmure: Linux package `plumed`, `/usr/bin/plumed`, resources, ONNX Runtime and the pacman marker in `/usr/lib/Plumed/` (the binary's rpath, set in `build-linux.yml`, points there), udev rule `60-plumed-uinput.rules`, release files `Plumed_*`. Upstream's crate names (`murmure`, `murmure_lib`) and the `.murmure` config import format are kept.

Auto-updater (`plugins.updater` in `src-tauri/tauri.conf.json`): the app downloads `latest.json` from our latest GitHub release and only installs an update whose signature matches the `pubkey` in that file. One-time setup:

1. `pnpm tauri signer generate -w ~/.tauri/murmuremed.key` (asks for a password). It writes the private key `murmuremed.key` and the public key `murmuremed.key.pub`. Keep the private key out of the repo and back it up: if it is lost, installed copies can never be updated again.
2. Paste the content of `murmuremed.key.pub` as `pubkey`.
3. In the GitHub repo settings (Secrets and variables > Actions), add `TAURI_PRIVATE_KEY` (the output of `base64 -w0 ~/.tauri/murmuremed.key`: the workflows decode it with `base64 -d` before signing, so the raw file content does not work) and `TAURI_KEY_PASSWORD`. The build workflows use them to sign the bundles (`.sig` files next to each installer).

Each release: bump the version (section 5), run the `Release Plumed` workflow, create a GitHub release tagged with the version (for example `0.1.0`) from its artifacts, then run `python scripts/make-latest-json.py <artifacts folder> --notes "..."` and upload the `latest.json` it writes. That file lists, per platform, the installer URL and the content of its `.sig` (format: https://v2.tauri.app/plugin/updater/#static-json-file); the workflows do not generate it.

Code signing: there will never be an Apple developer account, so macOS builds are unsigned and not notarized (users must allow the app in System Settings > Privacy & Security, or run `xattr -dr com.apple.quarantine /Applications/Plumed.app`); the macOS workflows run their Apple certificate, DMG signing and notarization steps only when the `MACOS_CODESIGN_P12_B64` secret exists, and otherwise ad-hoc sign the `.app` (`APPLE_SIGNING_IDENTITY=-`), which Apple Silicon requires to run it at all. Windows: the SignPath step runs only when `SIGNPATH_API_TOKEN` exists; otherwise the installers are released unsigned (SmartScreen warning). Either way the Tauri updater signature (`.sig`) is always made.

WinGet (`.github/workflows/publish-winget.yml`, id `Olicorne.Plumed`, NSIS `-setup.exe` only): it runs on every published release, but it can only update a package that already exists in microsoft/winget-pkgs. The first version must be submitted once by hand (for example with `komac new` or `wingetcreate new`), then add a `WINGET_TOKEN` secret (a GitHub token allowed to open PRs on winget-pkgs).

## 7. Other

- Personal notes and reminders live in `TODO.local.md` (gitignored).
- Machine-specific notes (which OSes the maintainer can test on) live in `AGENTS.local.md` (gitignored): read it if present.
- When writing docs, mention which coding harness was used (Claude Code or pi).
