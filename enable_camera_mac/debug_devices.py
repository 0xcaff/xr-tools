#!/usr/bin/env python3
"""
Debug script to list all HID devices
"""

import hid

print("Enumerating all HID devices...")
print("=" * 80)

devices = hid.enumerate(0, 0)

if not devices:
    print("No HID devices found!")
else:
    print(f"Found {len(devices)} HID devices:\n")

    for i, device in enumerate(devices):
        print(f"Device {i}:")
        print(f"  Vendor ID:     0x{device['vendor_id']:04x} ({device['vendor_id']})")
        print(f"  Product ID:    0x{device['product_id']:04x} ({device['product_id']})")
        print(f"  Manufacturer:  {device.get('manufacturer_string', 'N/A')}")
        print(f"  Product:       {device.get('product_string', 'N/A')}")
        print(f"  Path:          {device['path']}")
        print(f"  Interface:     {device.get('interface_number', 'N/A')}")
        print()

print("=" * 80)

# Check for Xreal devices specifically
print("\nLooking for Xreal devices (Vendor ID 0x3318)...")
xreal_devices = [d for d in devices if d['vendor_id'] == 0x3318]

if xreal_devices:
    print(f"Found {len(xreal_devices)} Xreal device(s):")
    for device in xreal_devices:
        print(f"  Product ID: 0x{device['product_id']:04x}")
        print(f"  Product: {device.get('product_string', 'N/A')}")
        print(f"  Path: {device['path']}")
else:
    print("No Xreal devices found (Vendor ID 0x3318)")
