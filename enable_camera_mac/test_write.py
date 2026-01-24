#!/usr/bin/env python3
"""
Test writing to each interface
"""

import struct
import zlib
import hid

VENDOR_ID = 0x3318
PRODUCT_IDS = [0x0435, 0x0436, 0x0437, 0x0438]

COMMAND_ID = bytes([0xD3, 0x00])


def build_packet(command_id: bytes, payload: bytes) -> bytes:
    """Build the USB HID control message packet."""
    magic = 0xFD
    request_id = 0
    timestamp = 0
    unknown = bytes([0, 0, 0, 0, 0])

    checksummed_size = 11
    total_length = checksummed_size + len(payload)

    checksummed_part = struct.pack(
        '<H I I 2s 5s',
        total_length,
        request_id,
        timestamp,
        command_id,
        unknown
    )

    checksum = zlib.adler32(checksummed_part) & 0xFFFFFFFF

    header = struct.pack('<B I', magic, checksum) + checksummed_part
    packet = header + payload

    return packet


devices = hid.enumerate(VENDOR_ID, 0)
xreal_devices = [d for d in devices if d['product_id'] in PRODUCT_IDS]

payload = struct.pack('<I', (1 << 6) | (1 << 8))
packet = build_packet(COMMAND_ID, payload)

print(f"Testing packet ({len(packet)} bytes): {packet.hex()}\n")

for i, device in enumerate(xreal_devices):
    print(f"Testing Device {i} (Interface {device.get('interface_number', 'N/A')}):")
    
    try:
        # Try open_path
        dev = hid.device()
        dev.open_path(device['path'])
        print(f"  ✓ Opened with open_path")
        
        # Try write
        try:
            result = dev.write(packet)
            print(f"  Write result: {result}")
            
            if result > 0:
                # Try read
                response = dev.read(1024, timeout_ms=2000)
                print(f"  Read {len(response)} bytes: {bytes(response).hex()}")
        except Exception as e:
            print(f"  ✗ Write failed: {e}")
        
        dev.close()
        
    except Exception as e:
        print(f"  ✗ Failed: {e}")
    
    print()
