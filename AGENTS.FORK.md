# MurmureMed fork instructions

These instructions come from the MurmureMed maintainer and apply on top of [AGENTS.md](AGENTS.md) (the upstream Murmure guidelines, which still apply). Where the two conflict, this file wins.

This file was set up with Claude Code.

## 1. What this fork is

MurmureMed is a fork of [Murmure](https://github.com/Kieirra/murmure) specialized for **French medical dictation**. It ports the features of the medical mode of ParakeetWeb ([live](http://parakeetweb.olicorne.org/?mode=med), [code](https://github.com/thiswillbeyourgithub/parakeet_web), usually checked out next to this repo in `../parakeet_web`) into an easy to install desktop app, so dictation works in any application of the computer, not only in a browser.

Why a fork and not upstream PRs: see [Kieirra/murmure#498](https://github.com/Kieirra/murmure/issues/498). The upstream maintainer prefers that very large rule lists (~30,000 regex rules) and medical specific post-processing live in a fork. We still want to stay as close to upstream as possible and upstream every change that is generic enough.

## 2. Mandatory at the start of every session: check upstream

**Last checked upstream commit:** `4973812` (docs: update roadmap)
**Date of check:** 2026-11-11

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

Branches are created only with the user's approval.

- `main`: what we release. Equal to upstream + fork meta files (README, this file) + every finished feature, each merged in.
- `feat/<name>`: **one feature per branch**, branched from the upstream commit we last synced to (not from `main`), so that `git diff upstream/main...feat/<name>` shows exactly that feature and nothing else. This makes each feature:
    - easy to review and revert (merged into `main` with `git merge --no-ff`, one merge commit per feature),
    - easy to benchmark in isolation (build `upstream/main` vs `feat/<name>` and compare),
    - easy to propose upstream (open a PR straight from the branch).
- If a feature depends on another one (example: the medical phrase list needs the new phrase boosting engine), branch it from its parent feature branch and note the dependency in the table below.
- Syncing upstream: `git merge upstream/main` into `main` (never rebase `main`, it is published). Feature branches not yet merged: rebase them onto `upstream/main` (or merge if already shared).
- `git rerere` is enabled locally (`git config rerere.enabled true`) so a conflict resolved once is resolved automatically next time.
- Recurring conflicts and how to resolve them:
    - `README.md`: always keep ours.
    - Version fields (see section 5): take the new upstream version and reapply our suffix.

### Feature tracking

| Feature | Branch | Depends on | Status | Upstream PR |
| --- | --- | --- | --- | --- |
| UltiMed fine-tune as default model | `feat/ultimed-model` | | planned | |
| Regex engine supporting lookarounds (fancy-regex or similar) | `feat/regex-engine` | | planned | |
| Compile regex once, not on every transcription | `feat/regex-compile-once` | | planned | candidate |
| Precompiled medical regex list, separate from user rules | `feat/medical-regex` | `feat/regex-engine`, `feat/regex-compile-once` | planned | |
| ParakeetWeb phrase boosting engine | `feat/phrase-boosting` | | planned | |
| French medical phrase boosting list | `feat/medical-phrase-list` | `feat/phrase-boosting` | planned | |
| Branding (name, icon, explicit fork credit), toggles for extra regex and phrase boosting | `feat/branding` | | planned | |
| Remote microphone through an internet relay (or iroh) | `feat/remote-mic-relay` | | idea | |

Update this table when a branch is created, merged, or proposed upstream.

## 5. Versioning

Format: `<upstream version>-med.<n>`, for example `2.0.3-med.17`. `<n>` is our own counter and resets to 1 when the upstream base changes (`2.0.4-med.1`).

Why not `2.0.3.17`: Cargo, `package.json` and Tauri all require semver (exactly three numbers), so a fourth number fails to parse. A semver pre-release suffix is accepted everywhere and compares correctly (`med.9` < `med.17`, numerically). The `+med.17` build metadata form is not used because semver ignores it when comparing, so the auto-updater would never see a new version.

The version must be identical in `package.json`, `src-tauri/Cargo.toml` (and `Cargo.lock`) and `src-tauri/tauri.conf.json`. In `tauri.conf.json` also set `bundle.windows.wix.version` to the four-number form (`2.0.3.17`), since MSI does not accept text pre-release suffixes.

Caveat: Windows Installer ignores the fourth number when deciding whether an MSI is an upgrade, so `2.0.3.17` to `2.0.3.18` would not upgrade cleanly through the `.msi`. Ship the NSIS `-setup.exe` as the main Windows installer (it is also the one that installs without admin rights).

## 6. Other

- Personal notes and reminders live in `TODO.local.md` (gitignored).
- Machine-specific notes (which OSes the maintainer can test on) live in `AGENTS.local.md` (gitignored): read it if present.
- When writing docs, mention which coding harness was used (Claude Code or pi).
