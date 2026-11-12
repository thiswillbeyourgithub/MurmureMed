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
    - Version fields (see section 5): take the new upstream version and reapply our suffix.

### Feature tracking

| Feature | Branch | Depends on | Status | Upstream PR |
| --- | --- | --- | --- | --- |
| UltiMed fine-tune as default model (CI fetches the int8 export from Hugging Face, pinned and hash-checked, by `scripts/fetch-ultimed-model.sh`) | `feat/ultimed-model` | | merged (boosting keeps helping with UltiMed: drug term recall 56 to 78 of 107, same tuning) | not planned (fork only) |
| Regex engine supporting lookarounds (fancy-regex or similar) | `feat/regex-engine` | | not needed for now (the medical rules' only lookbehind, the title exclusions, is handled in code; InterHop rules have none) | |
| Compile regex once, not on every transcription | `feat/regex-compile-once` | | merged (11x to 15x faster rule application) | not planned (upstream is fine with compiling per transcription for its short lists, see #498) |
| Medical regex list (UltiMed drug and term fixes, editable in an advanced panel, separate from user rules) plus InterHop dictation rules seeded as default user rules | `feat/medical-regex` | | merged | not planned (fork only, see #498) |
| ParakeetWeb phrase boosting engine (hybrid: upstream boost for the user dictionary, ParakeetWeb trie for phrase lists, larger bonus wins where they overlap) | `feat/phrase-boosting` (from `medmain`) | | merged | |
| French medical phrase boosting list (bundled from ParakeetWeb, armed at model load, ~2 s trie build on the startup thread) | `feat/medical-phrase-list` | `feat/phrase-boosting` | merged | |
| Branding (name, purple theme, fork credits, local-only notice, own identifier, updater and winget; icon and toggles for extra regex and phrase boosting still to do) | `feat/branding` (from `medmain`) | | merged, in progress | |
| Remote microphone through an internet relay (or iroh) | `feat/remote-mic-relay` | | idea | |

Update this table when a branch is created, merged, or proposed upstream.

Note for `feat/medical-regex`: the `feat/regex-compile-once` cache (capped at 4096 patterns) is sized for user rules, including the 138 InterHop rules seeded by default. The ~42,000 drug and term rules do not use it: `src-tauri/src/medical/` indexes them by anchor word at startup and compiles each rule lazily, the first time a text could match it.

Note for `feat/phrase-boosting`: benchmarked on 200 UltiMed validation clips (ParakeetWeb's `benchmark_datasets/french_medical/`, padded with 0.25 s of silence like the app does; unpadded clips drift into English and give meaningless numbers). WER without boosting 12.3% (drugs) / 13.1% (dictionary). Upstream's boost fed the ~77k medical phrases raises it to 17.6% / 18.5%: its strength and rank gate suit a few user words, not a huge list. ParakeetWeb's trie with the list lowers it to 12.0% / 11.7%. Hybrid with the dictionary on top: 11.6% / 12.0%, and the most target terms recovered. The ignored test `phrase_boost::model::tests::bench_transcribe` is the harness.

## 5. Versioning

Format: `<upstream version>-med.<n>`, for example `2.0.3-med.17`. `<n>` is our own counter and resets to 1 when the upstream base changes (`2.0.4-med.1`).

Why not `2.0.3.17`: Cargo, `package.json` and Tauri all require semver (exactly three numbers), so a fourth number fails to parse. A semver pre-release suffix is accepted everywhere and compares correctly (`med.9` < `med.17`, numerically). The `+med.17` build metadata form is not used because semver ignores it when comparing, so the auto-updater would never see a new version.

The version must be identical in `package.json`, `src-tauri/Cargo.toml` (and `Cargo.lock`) and `src-tauri/tauri.conf.json`. In `tauri.conf.json` also set `bundle.windows.wix.version` to the four-number form (`2.0.3.17`), since MSI does not accept text pre-release suffixes.

Caveat: Windows Installer ignores the fourth number when deciding whether an MSI is an upgrade, so `2.0.3.17` to `2.0.3.18` would not upgrade cleanly through the `.msi`. Ship the NSIS `-setup.exe` as the main Windows installer (it is also the one that installs without admin rights).

## 6. Releases

Identity: the app identifier is `org.olicorne.plumed` (own settings and history folder, separate from an installed Murmure). The product and binary name are still upstream's `murmure`, so the Linux `.deb` package has the same name as Murmure's and replaces it, and the macOS `.app` and Windows install folder collide with Murmure's too.

Auto-updater (`plugins.updater` in `src-tauri/tauri.conf.json`): the app downloads `latest.json` from our latest GitHub release and only installs an update whose signature matches the `pubkey` in that file. One-time setup:

1. `pnpm tauri signer generate -w ~/.tauri/murmuremed.key` (asks for a password). It writes the private key `murmuremed.key` and the public key `murmuremed.key.pub`. Keep the private key out of the repo and back it up: if it is lost, installed copies can never be updated again.
2. Paste the content of `murmuremed.key.pub` as `pubkey`.
3. In the GitHub repo settings (Secrets and variables > Actions), add `TAURI_PRIVATE_KEY` (content of `murmuremed.key`) and `TAURI_KEY_PASSWORD`. The build workflows use them to sign the bundles (`.sig` files next to each installer).

Each release: run the `Release Murmure` workflow, create a GitHub release from its artifacts, and upload a `latest.json` listing, per platform, the installer URL and the content of its `.sig` (format: https://v2.tauri.app/plugin/updater/#static-json-file). Upstream writes it by hand; the workflows do not generate it.

WinGet (`.github/workflows/publish-winget.yml`, id `Olicorne.Plumed`, NSIS `-setup.exe` only): it runs on every published release, but it can only update a package that already exists in microsoft/winget-pkgs. The first version must be submitted once by hand (for example with `komac new` or `wingetcreate new`), then add a `WINGET_TOKEN` secret (a GitHub token allowed to open PRs on winget-pkgs).

## 7. Other

- Personal notes and reminders live in `TODO.local.md` (gitignored).
- Machine-specific notes (which OSes the maintainer can test on) live in `AGENTS.local.md` (gitignored): read it if present.
- When writing docs, mention which coding harness was used (Claude Code or pi).
