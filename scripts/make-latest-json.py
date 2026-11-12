"""Plumed fork (written with Claude Code): build the updater's latest.json.

The installed app downloads latest.json from the latest GitHub release to learn
whether a newer version exists, where its installer is, and the minisign
signature that proves it was built with our private key (checked against
`plugins.updater.pubkey` in src-tauri/tauri.conf.json).

Usage, after downloading the release workflow's artifacts into one folder
(subfolders are fine):

    python scripts/make-latest-json.py ARTIFACTS_DIR [--tag 0.1.0] [--notes "..."]

It writes ARTIFACTS_DIR/latest.json; upload it to the GitHub release next to
the installers. The version comes from package.json, the tag defaults to it,
and the repository comes from the updater endpoint in tauri.conf.json.
Platforms whose installer or .sig is missing (for example no macOS build) are
left out with a warning.
"""

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Updater platform key -> installer file name produced by the build workflows.
# The keys without an installer suffix are the fallbacks Tauri uses when it
# does not know how the app was installed; on Windows we prefer NSIS, our main
# installer (see AGENTS.FORK.md, Versioning).
PLATFORMS = {
    "linux-x86_64": "Plumed_amd64.AppImage",
    "linux-x86_64-appimage": "Plumed_amd64.AppImage",
    "linux-x86_64-deb": "Plumed_amd64.deb",
    "linux-x86_64-rpm": "Plumed_amd64.rpm",
    "windows-x86_64": "Plumed_x64-setup.exe",
    "windows-x86_64-nsis": "Plumed_x64-setup.exe",
    "windows-x86_64-wix": "Plumed_x64.msi",
    "darwin-aarch64": "Plumed_aarch64_darwin.app.tar.gz",
    "darwin-x86_64": "Plumed_x86_64_darwin.app.tar.gz",
}


def release_download_base(tag: str) -> str:
    """Turn the updater endpoint (.../releases/latest/download/latest.json) into .../releases/download/<tag>."""
    conf = json.loads((ROOT / "src-tauri" / "tauri.conf.json").read_text())
    endpoint = conf["plugins"]["updater"]["endpoints"][0]
    repo, sep, _ = endpoint.partition("/releases/")
    if sep == "":
        sys.exit(f"Unexpected updater endpoint (no /releases/): {endpoint}")
    return f"{repo}/releases/download/{tag}"


def find_one(artifacts: Path, name: str) -> Path | None:
    matches = sorted(artifacts.rglob(name))
    if len(matches) > 1:
        sys.exit(f"Several files named {name} in {artifacts}: {matches}")
    return matches[0] if len(matches) == 1 else None


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("artifacts", type=Path, help="folder holding the installers and their .sig files")
    parser.add_argument("--tag", help="release tag (default: the version in package.json)")
    parser.add_argument("--notes", default="", help="release notes shown in the update dialog")
    args = parser.parse_args()

    version = json.loads((ROOT / "package.json").read_text())["version"]
    base = release_download_base(args.tag or version)

    platforms = {}
    for key, name in PLATFORMS.items():
        installer = find_one(args.artifacts, name)
        signature = find_one(args.artifacts, name + ".sig")
        if installer is None or signature is None:
            print(f"warning: {key} skipped, {name} or its .sig not found", file=sys.stderr)
            continue
        platforms[key] = {"signature": signature.read_text().strip(), "url": f"{base}/{name}"}

    if len(platforms) == 0:
        sys.exit(f"No signed installer found in {args.artifacts}")

    latest = {
        "version": version,
        "notes": args.notes,
        "pub_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": platforms,
    }
    out = args.artifacts / "latest.json"
    out.write_text(json.dumps(latest, indent=2) + "\n")
    print(f"wrote {out} ({len(platforms)} platforms, version {version})")


if __name__ == "__main__":
    main()
