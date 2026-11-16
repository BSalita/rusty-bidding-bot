#!/usr/bin/env python3
"""Copy a convention pack's published snapshot into pack/pack.json, the copy
of the rules compiled into rbb (crates/assets).

    python3 scripts/sync_pack.py                 # the `published` branch
    python3 scripts/sync_pack.py <directory>     # holding pack.json and manifest.json

The snapshot comes from bridge-common-conventions: its `published` branch, or
`python3 scripts/build_snapshot.py --out DIR` in a checkout. The copy leaves
out the test and notes files, which the engine does not bid with, and records
the commit and rules number the snapshot was built from.

The compiled-in copy is the fallback and the default for `rbb` with no
`--rules`; a host hands the engine a newer snapshot when it has one. Refresh
it at each release, not at each rule change.
"""
import json
import pathlib
import sys
import urllib.request

PUBLISHED = "https://raw.githubusercontent.com/bridge-craftwork/bridge-common-conventions/published/"
KEPT = (".bid", ".toml")


def read(source, name):
    if source is None:
        with urllib.request.urlopen(PUBLISHED + name) as r:
            return json.load(r)
    return json.loads((pathlib.Path(source) / name).read_text())


def main():
    source = sys.argv[1] if len(sys.argv) > 1 else None
    pack = read(source, "pack.json")
    manifest = read(source, "manifest.json")
    if manifest.get("uncommitted_changes"):
        print("warning: this snapshot was built from uncommitted changes", file=sys.stderr)
    files = {n: t for n, t in pack.pop("files").items() if n.endswith(KEPT)}
    out = {
        "generated_from": "bridge-common-conventions pack.json (do not edit: python3 scripts/sync_pack.py)",
        "source_commit": manifest["source_commit"],
        "rules_number": manifest["rules_number"],
        **pack,
        "files": files,
    }
    target = pathlib.Path(__file__).resolve().parent.parent / "pack" / "pack.json"
    target.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    rules = sum(1 for n in files if n.endswith(".bid"))
    print(f"pack/pack.json: rules number {out['rules_number']}, {rules} rule files, from {out['source_commit']}")


if __name__ == "__main__":
    main()
