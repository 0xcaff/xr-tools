#!/usr/bin/env python3
import argparse
import hashlib
import json
from pathlib import Path
import random
import re
from urllib.parse import urljoin, urlparse

import requests

# Viture firmware code mapping notes (sourced from Viture's updater JS):
# - N6   -> VITURE One XR Glasses
# - N6C  -> VITURE One Lite XR Glasses
# - N6P* -> VITURE Pro XR Glasses family
#   - N6PS, N6PL are Pro variants (inferred from n6p* firmware filenames)
# - P6C  -> VITURE Luma XR Glasses
# - P6   -> VITURE Luma Pro XR Glasses
# - P6S  -> VITURE Luma Ultra XR Glasses
# - P6X  -> VITURE x Cyberpunk 2077 Luma Cyber XR Glasses
# - R6   -> VITURE Beast XR Glasses
# - P9   -> VITURE Pro Mobile Dock track (separate updater mode)
# Notes:
# - N6S/N6T/N6D, P6D, P6S+, and R6D appear in the live feed as hardware/variant
#   codes; Viture does not publish all of their marketing names in the public
#   `vers.js` payload.


def safePathComponent(value: str) -> str:
    if "/" in value:
        raise ValueError(f"Unexpected '/' in path component: {value!r}")
    return value


def parseFirmwarePayload(source: str) -> dict:
    match = re.fullmatch(
        r"\s*(?:var|let|const)\s+firmware_vers\s*=\s*(\{.*\})\s*;?\s*",
        source,
        flags=re.S,
    )
    if not match:
        raise ValueError("Unexpected payload format")

    data = json.loads(match.group(1))
    if not isinstance(data, dict):
        raise RuntimeError("Expected firmware_vers to be a JSON object.")
    return data


def downloadFile(url: str, dest: Path) -> str:
    sha256 = hashlib.sha256()
    with requests.get(url, stream=True, timeout=120) as resp:
        resp.raise_for_status()
        with dest.open("wb") as out:
            for chunk in resp.iter_content(chunk_size=1024 * 1024):
                if chunk:
                    out.write(chunk)
                    sha256.update(chunk)
    return sha256.hexdigest()


def stageModelFirmware(output_root: Path, model_code: str, model_payload: dict) -> None:
    ver_name = model_payload.get("verName")
    if not ver_name:
        raise RuntimeError(f"Missing verName for model {model_code}")

    relative_url = model_payload.get("url")
    if not relative_url:
        raise RuntimeError(f"Missing firmware url for model {model_code}")

    version_dir = output_root / safePathComponent(model_code) / safePathComponent(
        ver_name
    )
    version_json = version_dir / "version.json"
    if version_json.is_file():
        print(f"Already staged: {version_json}")
        return

    firmware_url = urljoin("https://static.viture.com", relative_url)
    filename = Path(urlparse(firmware_url).path).name or "firmware.bin"
    firmware_dest = version_dir / "firmware" / filename
    firmware_dest.parent.mkdir(parents=True, exist_ok=True)

    sha256 = downloadFile(firmware_url, firmware_dest)

    staged_payload = {
        "modelCode": model_code,
        "firmwareUrl": firmware_url,
        "firmwareFileName": filename,
        "firmwareSha256": sha256,
        "firmwarePayload": model_payload,
    }
    version_dir.mkdir(parents=True, exist_ok=True)
    with version_json.open("w", encoding="utf-8") as f:
        json.dump(staged_payload, f, ensure_ascii=False)

    print(f"Staged {model_code} {ver_name} in {version_dir}")


def run(args):
    output_root = Path(args.output).expanduser().resolve()
    output_root.mkdir(parents=True, exist_ok=True)

    # Mirror the web updater behavior and add a cache-busting parameter.
    resp = requests.get(
        "https://static.viture.com/dfu-util/js/vers.js",
        params={"ord": int(random.random() * 10_000_000_000_000_000_000)},
        timeout=30,
    )
    resp.raise_for_status()
    versions = parseFirmwarePayload(resp.text)

    for model_code, model_payload in sorted(versions.items()):
        try:
            stageModelFirmware(output_root, model_code, model_payload)
        except Exception as err:  # noqa: BLE001
            print(f"Failed to stage {model_code}: {err}")


def main():
    parser = argparse.ArgumentParser(description="Fetch and stage Viture firmware assets.")
    parser.add_argument("output", help="Destination root directory")
    run(parser.parse_args())


if __name__ == "__main__":
    main()
