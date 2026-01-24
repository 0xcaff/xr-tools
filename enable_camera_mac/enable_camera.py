#!/usr/bin/env python3
"""
Enable Xreal One Pro camera on macOS

This script sends a USB HID command to enable the UVC camera interface.
After running, the device will reconnect with a standard webcam interface.
"""

import traceback
import struct
import zlib
import hid

# USB device identifiers
VENDOR_ID = 0x3318
PRODUCT_IDS = [0x0435, 0x0436, 0x0437, 0x0438]  # Xreal One and One Pro

# Command ID for setting USB configuration
COMMAND_ID = bytes([0xD3, 0x00])


def build_packet(command_id: bytes, payload: bytes) -> bytes:
    """Build the USB HID control message packet."""
    magic = 0xFD
    request_id = 0
    timestamp = 0
    unknown = bytes([0, 0, 0, 0, 0])

    # Calculate header sizes
    # checksummed_part = length(2) + request_id(4) + timestamp(4) + command(2) + unknown(5) = 17 bytes
    checksummed_size = 17
    total_length = checksummed_size + len(payload)

    # Build the checksummed part (everything after magic and checksum)
    checksummed_part = struct.pack(
        '<H I I 2s 5s',
        total_length,
        request_id,
        timestamp,
        command_id,
        unknown
    )

    # Calculate checksum over header fields + payload (as Rust code does)
    checksum = zlib.crc32(checksummed_part + payload) & 0xFFFFFFFF

    # Build full header
    header = struct.pack('<B I', magic, checksum) + checksummed_part

    # Full packet = header + payload
    packet = header + payload

    return packet


def find_device():
    """Find the Xreal One Pro device on interface 0 (the one that accepts writes)."""
    devices = hid.enumerate(VENDOR_ID, 0)

    for device in devices:
        if device['product_id'] in PRODUCT_IDS and device.get('interface_number') == 0:
            print(f"Found device: {device['product_string']}")
            print(f"  Vendor ID: 0x{device['vendor_id']:04x}")
            print(f"  Product ID: 0x{device['product_id']:04x}")
            print(f"  Interface: {device['interface_number']}")
            print(f"  Path: {device['path']}")
            return device

    raise RuntimeError("Xreal One Pro device not found. Make sure it's connected.")


def enable_camera():
    """Enable the camera on Xreal One Pro."""
    # Find and open the device
    device_info = find_device()
    device = hid.device()
    device.open_path(device_info['path'])

    try:
        print("\nDevice opened successfully")

        # Build the USB config payload
        # UsbConfigList is a 32-bit bitfield with 2-bit fields:
        # - ncm: bits 0-1, ecm: bits 2-3, uac: bits 4-5, hid_ctrl: bits 6-7
        # - mtp: bits 8-9, mass_storage: bits 10-11, uvc0: bits 12-13, uvc1: bits 14-15
        # - enable: bits 16-17
        # Values: 0=no change, 1=enable, 2=disable
        uvc0 = 1 << 12   # uvc0=1 (enable camera)
        enable = 1 << 16  # enable=1 (apply config)
        payload = struct.pack('<I', uvc0 | enable)

        print(f"Payload: {payload.hex()}")

        # Build the full packet
        packet = build_packet(COMMAND_ID, payload)
        print(f"Packet ({len(packet)} bytes): {packet.hex()}")

        # Send the command
        print("\nSending command to enable camera...")
        bytes_written = device.write(packet)
        print(f"Wrote {bytes_written} bytes")

        # Read response
        print("Reading response...")
        response = device.read(1024, timeout_ms=5000)
        print(f"Received {len(response)} bytes: {bytes(response).hex()}")

        # Check response
        if bytes_written > 0 and len(response) >= 23:
            response_magic = response[0]
            response_status = response[22]  # Status byte is at index 22 (after 22-byte header)

            if response_magic == 0xFD:
                print(f"\n✓ Response magic valid (0xFD)")
                if response_status == 0:
                    print("✓ Command successful!")
                    print("\n📷 Camera enabled!")
                    print("The device will now reconnect with a UVC camera interface.")
                    print("You can use standard webcam tools (ffmpeg, QuickTime, etc.) to capture images.")
                else:
                    print(f"✗ Command failed with status: {response_status}")
            else:
                print(f"✗ Invalid response magic: 0x{response_magic:02x}")
        else:
            print(f"✗ Write failed or invalid response (wrote: {bytes_written}, response len: {len(response)})")

    finally:
        device.close()
        print("\nDevice closed")


if __name__ == '__main__':
    try:
        enable_camera()
    except Exception as e:
        print(f"\n✗ Error: {e}")
        traceback.print_exc()
