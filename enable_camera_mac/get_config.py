#!/usr/bin/env python3
"""
Get current USB config from Xreal One Pro
"""

import struct
import zlib
import hid

VENDOR_ID = 0x3318
PRODUCT_IDS = [0x0435, 0x0436, 0x0437, 0x0438]

# Command ID for getting USB configuration
GET_CONFIG_COMMAND_ID = bytes([0xD2, 0x00])


def build_packet(command_id: bytes, payload: bytes = b'') -> bytes:
    """Build the USB HID control message packet."""
    magic = 0xFD
    request_id = 0
    timestamp = 0
    unknown = bytes([0, 0, 0, 0, 0])

    checksummed_size = 17
    total_length = checksummed_size + len(payload)

    checksummed_part = struct.pack(
        '<H I I 2s 5s',
        total_length,
        request_id,
        timestamp,
        command_id,
        unknown
    )

    checksum = zlib.crc32(checksummed_part + payload) & 0xFFFFFFFF

    header = struct.pack('<B I', magic, checksum) + checksummed_part
    packet = header + payload

    return packet


def parse_usb_config(config_bytes: int) -> dict:
    """Parse UsbConfigList bitfield."""
    return {
        'ncm': (config_bytes >> 0) & 0x3,
        'ecm': (config_bytes >> 2) & 0x3,
        'uac': (config_bytes >> 4) & 0x3,
        'hid_ctrl': (config_bytes >> 6) & 0x3,
        'mtp': (config_bytes >> 8) & 0x3,
        'mass_storage': (config_bytes >> 10) & 0x3,
        'uvc0': (config_bytes >> 12) & 0x3,
        'uvc1': (config_bytes >> 14) & 0x3,
        'enable': (config_bytes >> 16) & 0x3,
    }


def get_usb_config():
    """Get current USB config from device."""
    devices = hid.enumerate(VENDOR_ID, 0)

    device_info = None
    for d in devices:
        if d['product_id'] in PRODUCT_IDS and d.get('interface_number') == 0:
            device_info = d
            break

    if not device_info:
        raise RuntimeError("Xreal One Pro device not found")

    print(f"Found device: {device_info['product_string']}")
    print(f"  Interface: {device_info['interface_number']}")

    dev = hid.device()
    dev.open_path(device_info['path'])

    try:
        packet = build_packet(GET_CONFIG_COMMAND_ID)
        print(f"\nSending GET_USB_CONFIG command...")
        print(f"Packet ({len(packet)} bytes): {packet.hex()}")

        bytes_written = dev.write(packet)
        print(f"Wrote {bytes_written} bytes")

        response = dev.read(1024, timeout_ms=2000)
        print(f"Received {len(response)} bytes")
        print(f"Response hex: {bytes(response[:32]).hex()}...")

        if len(response) >= 27:
            magic = response[0]
            status = response[22]
            
            print(f"\nMagic: 0x{magic:02x}")
            print(f"Status: {status}")

            if magic == 0xFD and status == 0:
                # Config data starts at index 23 (after header + status)
                config_bytes = struct.unpack('<I', bytes(response[23:27]))[0]
                print(f"\nRaw config: 0x{config_bytes:08x}")
                
                config = parse_usb_config(config_bytes)
                print("\nUSB Configuration:")
                print(f"  ncm:          {config['ncm']} (0=off, 1=on, 2=disabled)")
                print(f"  ecm:          {config['ecm']}")
                print(f"  uac:          {config['uac']}")
                print(f"  hid_ctrl:     {config['hid_ctrl']}")
                print(f"  mtp:          {config['mtp']}")
                print(f"  mass_storage: {config['mass_storage']}")
                print(f"  uvc0:         {config['uvc0']} ← camera")
                print(f"  uvc1:         {config['uvc1']}")
                print(f"  enable:       {config['enable']}")
            else:
                print(f"Command failed")

    finally:
        dev.close()


if __name__ == '__main__':
    get_usb_config()
