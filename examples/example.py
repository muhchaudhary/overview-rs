#!/usr/bin/env python3
"""
Example: Capture a frame from a Hyprland window and save it as PNG

Usage:
    python example.py <window_handle>

Where window_handle is the decimal address from 'hyprctl clients'
"""

import sys
import numpy as np
from PIL import Image
from hyprland_overview_rs.hyprland_overview_rs import HyprlandFrameCapture


def main():
    if len(sys.argv) < 2:
        print("Usage: python example.py <window_handle>")
        print("\nGet window handles by running:")
        print("  hyprctl clients -j | jq -r '.[] | \"\\(.title): \\(.address)\"'")
        print("\nThen convert hex address to decimal:")
        print("  python -c 'print(int(\"0x559bf1bc4f80\", 16))'")
        sys.exit(1)
    
    window_handle = int(sys.argv[1], 16)
    
    print(f"Capturing window with handle: {window_handle} (0x{window_handle:x})")
    
    # Create capture instance
    capture = HyprlandFrameCapture()
    
    # Capture frame
    print("Capturing frame...")
    frame_data, frame_info = capture.capture_frame(window_handle, overlay_cursor=1)
    
    print(f"✓ Captured {len(frame_data)} bytes")
    print(f"  Dimensions: {frame_info.width}x{frame_info.height}")
    print(f"  Stride: {frame_info.stride}")
    print(f"  Format: {frame_info.format}")
    
    # Convert to numpy array (BGRA format)
    arr = np.frombuffer(frame_data, dtype=np.uint8)
    arr = arr.reshape((frame_info.height, frame_info.width, 4))
    
    # Convert BGRA to RGBA
    arr = arr[:, :, [2, 1, 0, 3]]  # Swap B and R channels
    
    # Create PIL Image and save
    img = Image.fromarray(arr, 'RGBA')
    output_file = "screenshot.png"
    img.save(output_file)
    
    print(f"✓ Saved to {output_file}")


if __name__ == "__main__":
    main()
