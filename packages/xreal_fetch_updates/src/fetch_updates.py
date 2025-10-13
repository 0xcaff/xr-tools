#!/usr/bin/env python3
import argparse
import hashlib
from pathlib import Path
import requests
import urllib.parse

BASE_URL = "https://app-api.xreal.com/api/nebula/v1/isc/device/package"


def run(args):
    hardware_codes = [
        2, # xreal air

        # xreal air 2 and xreal air 2 pro
        3,
        4,

        5, # xreal air 2 ultra

        6, # xreal one pro
        7, # xreal one
    ]


    for hardware_code in hardware_codes:
        params = {
            "packageName": "ai.nreal.web",
            "hardwareCode": hardware_code,
            "versionCode": 1
        }

        url = f"{BASE_URL}?{urllib.parse.urlencode(params)}"

        output_root = Path(args.output).expanduser().resolve()

        resp = requests.get(url, timeout=30)
        resp.raise_for_status()
        raw_text = resp.text
        doc = resp.json()

        data = doc["data"]
        ident = data["identification"]
        version_dir = output_root / str(hardware_code) / ident
        version_json = version_dir / "version.json"

        if version_json.is_file():
            print(f"Already staged: {version_json}")
            return

        # Download each file to OUTPUT/{hardware_code}/{ident}/{fileType}/{fileName} and verify md5
        for f in data["files"]:
            file_type = f["fileType"]
            file_name = f["fileName"]
            url = f["filePath"]
            expected_md5 = f["checksum"].lower()

            dest = version_dir / file_type / file_name
            dest.parent.mkdir(parents=True, exist_ok=True)

            with requests.get(url, stream=True, timeout=60) as r:
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

        # Write raw JSON text exactly as received
        version_dir.mkdir(parents=True, exist_ok=True)
        with version_json.open("w", encoding="utf-8") as f:
            f.write(raw_text)

        print(f"Staged {ident} in {version_dir}")


def main():
    parser = argparse.ArgumentParser(description="Fetch and stage firmware assets.")
    parser.add_argument("output", help="Destination root directory")
    args = parser.parse_args()
    run(args)


if __name__ == "__main__":
    main()
