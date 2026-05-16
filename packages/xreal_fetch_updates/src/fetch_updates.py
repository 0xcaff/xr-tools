#!/usr/bin/env python3
import argparse
from dataclasses import dataclass
from enum import IntEnum
from pathlib import Path
import requests
import urllib.parse

from package_utils import save_package


class HardwareCode(IntEnum):
    XREAL_AIR = 2
    XREAL_AIR_2 = 3
    XREAL_AIR_2_PRO = 4
    XREAL_AIR_2_ULTRA = 5
    XREAL_ONE_PRO = 6
    XREAL_ONE = 7
    XREAL_UNKNOWN_9 = 9


@dataclass(frozen=True)
class PackageTarget:
    package_name: str
    hardware_codes: list[HardwareCode]


PACKAGE_TARGETS = [
    PackageTarget(
        "ai.nreal.web",
        [
            HardwareCode.XREAL_AIR,
            HardwareCode.XREAL_AIR_2,
            HardwareCode.XREAL_AIR_2_PRO,
            HardwareCode.XREAL_AIR_2_ULTRA,
            HardwareCode.XREAL_ONE_PRO,
            HardwareCode.XREAL_ONE,
            HardwareCode.XREAL_UNKNOWN_9,
        ],
    ),
    PackageTarget(
        "com.xreal.web.recovery",
        [
            HardwareCode.XREAL_ONE_PRO,
            HardwareCode.XREAL_ONE,
            HardwareCode.XREAL_UNKNOWN_9,
        ],
    ),
]


def run(args):
    output_root = Path(args.output).expanduser().resolve()
    for target in PACKAGE_TARGETS:
        for hardware_code in target.hardware_codes:
            params = {
                "packageName": target.package_name,
                "hardwareCode": int(hardware_code),
                "versionCode": 1,
            }
            url = (
                "https://app-api.xreal.com/api/nebula/v1/isc/device/package"
                f"?{urllib.parse.urlencode(params)}"
            )
            resp = requests.get(url, timeout=30)
            resp.raise_for_status()
            doc = resp.json()
            save_package(doc, output_root / target.package_name, int(hardware_code))


def main():
    parser = argparse.ArgumentParser(description="Fetch and stage firmware assets.")
    parser.add_argument("output", help="Destination root directory")
    args = parser.parse_args()
    run(args)


if __name__ == "__main__":
    main()
