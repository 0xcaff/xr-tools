#!/usr/bin/env python3
import hashlib
import json

import requests


def save_package(doc, output_root, hardware_code):
    data = doc["data"]
    ident = data["identification"]
    version_dir = output_root / str(hardware_code) / ident
    version_json = version_dir / "version.json"

    if version_json.is_file():
        print(f"Already staged: {version_json}")
        return

    # Download each file to OUTPUT/{hardware_code}/{ident}/{fileType}/{fileName} and verify md5
    for f in data["files"]:
        expected_md5 = f["checksum"].lower()

        dest = version_dir / f["fileType"] / f["fileName"]
        dest.parent.mkdir(parents=True, exist_ok=True)

        with requests.get(f["filePath"], stream=True, timeout=60) as r:
            r.raise_for_status()
            with dest.open("wb") as out:
                for chunk in r.iter_content(chunk_size=1024 * 1024):
                    if chunk:
                        out.write(chunk)

        md5 = hashlib.md5()
        with dest.open("rb") as fh:
            for chunk in iter(lambda: fh.read(1024 * 1024), b""):
                md5.update(chunk)
        actual_md5 = md5.hexdigest().lower()

        if actual_md5 != expected_md5:
            raise RuntimeError(
                f"Checksum mismatch for {dest.name}: expected {expected_md5}, got {actual_md5}"
            )

    # Write the package payload as JSON for reproducibility
    version_dir.mkdir(parents=True, exist_ok=True)
    with version_json.open("w", encoding="utf-8") as f:
        f.write(json.dumps(data))

    print(f"Staged {ident} in {version_dir}")
