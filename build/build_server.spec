"""
PyInstaller spec: wechat_mcp_server.exe
MCP stdio 服务器，无需管理员权限，由 Claude Desktop/Code 直接调用。
"""
import sys
from PyInstaller.utils.hooks import collect_submodules, collect_data_files

block_cipher = None

hiddenimports = (
    collect_submodules('mcp') +
    collect_submodules('Crypto') +
    collect_submodules('Cryptodome') +
    collect_submodules('zstandard') +
    collect_submodules('anyio') +
    collect_submodules('sniffio') +
    collect_submodules('starlette') +
    collect_submodules('uvicorn') +
    ['decode_image', 'config']
)

datas = collect_data_files('mcp')

a = Analysis(
    ['../mcp_server.py'],
    pathex=['..'],
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='wechat_mcp_server',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=True,   # stdio transport — must be console=True
    uac_admin=False,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=False,
    name='wechat_mcp_server',
)
