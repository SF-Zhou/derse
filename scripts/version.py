#!/usr/bin/env python3
"""Check or update the workspace release version; requires Python 3.8+ and Cargo."""

import argparse
import json
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parents[1]
PUBLISHED = {"derse", "derse-derive"}
VERSION = re.compile(
    r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)


def check():
    """Ask Cargo to resolve inheritance, then check the release policy."""
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    metadata = json.loads(result.stdout)
    members = {
        package["name"]: package
        for package in metadata["packages"]
        if package["id"] in metadata["workspace_members"]
    }
    if {name for name, package in members.items() if package["publish"] != []} != PUBLISHED:
        raise ValueError("only derse and derse-derive may be published")
    derive_dependencies = [
        dependency for dependency in members["derse"]["dependencies"]
        if dependency["name"] == "derse-derive" and dependency["kind"] is None
    ]
    if len(derive_dependencies) != 1:
        raise ValueError("derse must depend on the matching derse-derive runtime dependency")
    # Cargo reports req="*" for both path-only and explicit wildcard requirements.
    # Only the path-only form is omitted from the published macro manifest.
    derive_manifest = Path(members["derse-derive"]["manifest_path"]).read_text()
    if not re.search(r'(?m)^derse\s*=\s*\{\s*path\s*=\s*"\.\./derse"\s*\}\s*$', derive_manifest):
        raise ValueError('keep the derive dev-dependency as derse = { path = "../derse" }')
    version = members["derse"]["version"]
    for name, package in members.items():
        manifest = Path(package["manifest_path"]).read_text()
        if not re.search(r"(?m)^version\.workspace\s*=\s*true\s*$", manifest):
            raise ValueError(name + " must inherit workspace.package.version")
        if package["version"] != version:
            raise ValueError(name + " has a different workspace version")
        for dependency in package["dependencies"]:
            if dependency["name"] not in members:
                continue
            if dependency.get("path") is None:
                raise ValueError(name + " must use local workspace dependencies")
            if name in PUBLISHED and dependency["kind"] != "dev":
                if dependency["req"] != "=" + version:
                    raise ValueError(name + " must pin internal dependencies to =" + version)
            elif name in PUBLISHED and dependency["req"] != "*":
                raise ValueError("keep the derive runtime dev-dependency path-only to avoid a release cycle")
    readme = (ROOT / "README.md").read_text()
    if not re.search(r'(?m)^derse = "=' + re.escape(version) + r'"$', readme):
        raise ValueError("README installation version differs from the workspace")
    print("Workspace version: " + version + "; publish: " + ", ".join(sorted(PUBLISHED)))


def replace_once(pattern, replacement, text):
    updated, count = re.subn(pattern, lambda match: replacement(match), text, flags=re.MULTILINE)
    if count != 1:
        raise ValueError("expected one version declaration matching " + pattern)
    return updated


def set_version(version):
    # Build metadata is deliberately excluded: it cannot select a distinct release
    # in Cargo dependency requirements. Numeric prerelease identifiers follow SemVer.
    if not VERSION.fullmatch(version):
        raise ValueError("use a SemVer version without build metadata, e.g. 0.2.0-alpha.1 or 0.2.0")
    prerelease = version.partition("-")[2]
    if any(part.isdigit() and len(part) > 1 and part[0] == "0" for part in prerelease.split(".")):
        raise ValueError("numeric prerelease identifiers cannot have leading zeros")

    manifest_path = ROOT / "Cargo.toml"
    readme_path = ROOT / "README.md"
    originals = {path: path.read_text() for path in (manifest_path, readme_path)}
    manifest = replace_once(
        r'^(version = ")[^"]+(".*)$',
        lambda match: match[1] + version + match[2],
        originals[manifest_path],
    )
    manifest = replace_once(
        r'^(derse-derive = \{ version = "=)[^"]+(".*)$',
        lambda match: match[1] + version + match[2],
        manifest,
    )
    readme = replace_once(
        r'^derse = "=[^"]+"$',
        lambda _: 'derse = "=' + version + '"',
        originals[readme_path],
    )
    try:
        manifest_path.write_text(manifest)
        readme_path.write_text(readme)
        check()
    except (OSError, ValueError, subprocess.CalledProcessError):
        for path, original in originals.items():
            path.write_text(original)
        raise
    print("Review CHANGELOG.md and run the checks in docs/releasing.md before publishing.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("check", help="check versions, internal dependencies, and publish selection")
    update = commands.add_parser("set", help="update the workspace, derive requirement, and README")
    update.add_argument("version")
    args = parser.parse_args()
    try:
        if args.command == "check":
            check()
        else:
            set_version(args.version)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, "version: " + str(error) + "\n")


if __name__ == "__main__":
    main()
