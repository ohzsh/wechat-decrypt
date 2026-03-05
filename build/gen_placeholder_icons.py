"""Generate minimal placeholder icons for tauri-build validation in CI."""
from PIL import Image
import os

os.makedirs('src-tauri/icons', exist_ok=True)
img = Image.new('RGBA', (512, 512), (9, 187, 7, 255))
img.save('src-tauri/icons/icon.png')
img.save('src-tauri/icons/icon.ico')
print("Placeholder icons written to src-tauri/icons/")
