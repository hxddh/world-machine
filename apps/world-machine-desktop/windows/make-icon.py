#!/usr/bin/env python3
"""Writes the Windows icon (.ico, 16 to 256 pixels) from the app's 1024px
source, apps/world-machine-desktop/macos/icon.png. Needs Pillow."""

import sys
from pathlib import Path

from PIL import Image

source, destination = Path(sys.argv[1]), Path(sys.argv[2])
sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
Image.open(source).convert("RGBA").save(destination, format="ICO", sizes=sizes)
