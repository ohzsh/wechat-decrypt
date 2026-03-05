"""
生成 WeChat MCP 应用图标。
需要 Pillow: pip install pillow

生成后运行: cargo tauri icon assets/icon.png
（会自动生成所有 src-tauri/icons/ 文件）
"""
import os
import sys

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    print("需要 Pillow: pip install pillow")
    sys.exit(1)

SIZE = 512

img = Image.new('RGBA', (SIZE, SIZE), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)

# Background circle
draw.ellipse([8, 8, SIZE - 8, SIZE - 8], fill=(20, 20, 40, 255))

# WeChat green circle
draw.ellipse([40, 40, SIZE - 40, SIZE - 40], fill=(9, 187, 7, 255))

# Simple chat bubble shape
draw.ellipse([110, 130, 310, 290], fill=(255, 255, 255, 230))
draw.ellipse([200, 230, 400, 390], fill=(255, 255, 255, 200))

# Connection dot
draw.ellipse([220, 370, 260, 410], fill=(255, 255, 255, 230))

out = os.path.join(os.path.dirname(__file__), "assets", "icon.png")
os.makedirs(os.path.dirname(out), exist_ok=True)
img.save(out)
print(f"图标已生成: {out}")
print("接下来运行: cargo tauri icon assets/icon.png")
