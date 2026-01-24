# pycontrol

Python tools for enabling Xreal One Pro camera on macOS.
How it works?
1. It send a special packet to USB HID device
2. It will appear as UVC Camera 0

## Setup

```bash
poetry install
```

## Step 1 - Enable Camera

```bash
poetry run python enable_camera.py
```

Expected output:
```
Found device: XREAL One Pro
...
✓ Command successful!
📷 Camera enabled!
```

## Step 2 - Verify

```bash
poetry run python get_config.py
```

Expected output should show `uvc0: 1`:
```
USB Configuration:
  ...
  uvc0:         1 ← camera
  ...
```

If `uvc0: 0`, run enable_camera.py again / run other debug scripts / report issue.
