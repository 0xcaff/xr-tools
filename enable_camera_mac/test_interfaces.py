#!/usr/bin/env python3
"""
Try opening each Xreal HID interface to see which one works
"""

import hid

VENDOR_ID = 0x3318
PRODUCT_IDS = [0x0435, 0x0436, 0x0437, 0x0438]

devices = hid.enumerate(VENDOR_ID, 0)

xreal_devices = [d for d in devices if d['product_id'] in PRODUCT_IDS]

print(f"Found {len(xreal_devices)} Xreal device(s):\n")

for i, device in enumerate(xreal_devices):
    print(f"Device {i}:")
    print(f"  Vendor ID:     0x{device['vendor_id']:04x}")
    print(f"  Product ID:    0x{device['product_id']:04x}")
    print(f"  Manufacturer:  {device.get('manufacturer_string', 'N/A')}")
    print(f"  Product:       {device.get('product_string', 'N/A')}")
    print(f"  Path:          {device['path']}")
    print(f"  Interface:     {device.get('interface_number', 'N/A')}")
    print(f"  Usage Page:    {device.get('usage_page', 'N/A')}")
    print(f"  Usage:         {device.get('usage', 'N/A')}")
    
    # Try to open it
    try:
        dev = hid.device()
        dev.open_path(device['path'])
        print(f"  ✓ Opened successfully")
        
        # Try to read something
        try:
            data = dev.read(64, timeout_ms=100)
            print(f"  Read {len(data)} bytes: {bytes(data).hex()}")
        except Exception as e:
            print(f"  Read failed: {e}")
        
        dev.close()
    except Exception as e:
        print(f"  ✗ Failed to open: {e}")
    
    print()
