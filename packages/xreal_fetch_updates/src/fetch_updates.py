#!/usr/bin/env python3
import argparse
from pathlib import Path
import requests
import urllib.parse

from package_utils import save_package


def run(args):
    output_root = Path(args.output).expanduser().resolve()
    hardware_codes = [
        2,  # xreal air
        # xreal air 2 and xreal air 2 pro
        3,
        4,
        5,  # xreal air 2 ultra
        6,  # xreal one pro
        7,  # xreal one
    ]

    for hardware_code in hardware_codes:
        params = {
            "packageName": "ai.nreal.web",
            "hardwareCode": hardware_code,
            "versionCode": 1,
        }

        url = (
            "https://app-api.xreal.com/api/nebula/v1/isc/device/package"
            f"?{urllib.parse.urlencode(params)}"
        )

        resp = requests.get(url, timeout=30)
        resp.raise_for_status()
        doc = resp.json()

        save_package(doc, output_root, hardware_code)


def main():
    parser = argparse.ArgumentParser(description="Fetch and stage firmware assets.")
    parser.add_argument("output", help="Destination root directory")
    args = parser.parse_args()
    run(args)


if __name__ == "__main__":
    main()
