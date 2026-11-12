from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: compute_rust_source_manifest.py <workspace-root> <output-json>")
    root = Path(sys.argv[1]).resolve(strict=True)
    output = Path(sys.argv[2]).resolve()
    files = sorted(
        (root / "crates").rglob("*.rs"),
        key=lambda path: path.relative_to(root).as_posix(),
    )
    records = []
    aggregate = hashlib.sha256()
    for path in files:
        relative = path.relative_to(root).as_posix()
        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        aggregate.update(relative.encode("utf-8") + b"\0")
        aggregate.update(bytes.fromhex(digest) + b"\0")
        records.append(
            {
                "path": relative,
                "bytes": len(data),
                "sha256": digest,
            }
        )
    payload = {
        "manifest_version": 1,
        "workspace_root": str(root),
        "file_count": len(records),
        "manifest_sha256": aggregate.hexdigest(),
        "files": records,
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()