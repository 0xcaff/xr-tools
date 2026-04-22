#!/usr/bin/env python3
import argparse
from pathlib import Path
import requests
import urllib.parse

from package_utils import save_package


def run_backfill(args):
    output_root = Path(args.output).expanduser().resolve()
    if args.backfill_min_version_code > args.backfill_max_version_code:
        raise ValueError("backfill min version code must be <= max version code")

    for version_code in range(
        args.backfill_min_version_code, args.backfill_max_version_code + 1
    ):
        params = {
            "packageName": args.backfill_package_name,
            "versionCode": version_code,
        }
        url = (
            "https://app-api.xreal.com/api/nebula/v1/isc/device/firmwares"
            f"?{urllib.parse.urlencode(params)}"
        )

        resp = requests.get(url, timeout=30)
        resp.raise_for_status()
        doc = resp.json()
        packages = (doc.get("data") or {}).get("firmwarePackages") or []

        for package in packages:
            hardware_code = package["hardwareCode"]
            save_package({"data": package}, output_root, hardware_code)


def main():
    parser = argparse.ArgumentParser(description="Backfill and stage firmware assets.")
    parser.add_argument("output", help="Destination root directory")
    parser.add_argument(
        "--backfill-package-name",
        default="com.xreal.evapro.nebula",
        help="Package name used for the backfill firmware endpoint.",
    )
    parser.add_argument(
        "--backfill-min-version-code",
        type=int,
        default=1,
        help="Smallest app versionCode to query when backfilling.",
    )
    parser.add_argument(
        "--backfill-max-version-code",
        type=int,
        default=3000,
        help="Largest app versionCode to query when backfilling.",
    )
    args = parser.parse_args()
    run_backfill(args)


if __name__ == "__main__":
    main()
