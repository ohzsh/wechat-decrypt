"""
PyInstaller spec: wechat_worker.exe
统一 CLI 工作进程：密钥提取（需要管理员）+ Web UI 启动。
uac_admin=True 使 Windows 在启动时自动弹出 UAC 确认框。
"""
from PyInstaller.utils.hooks import collect_submodules, collect_data_files

block_cipher = None

hiddenimports = (
    collect_submodules('Crypto') +
    collect_submodules('Cryptodome') +
    collect_submodules('zstandard') +
    collect_submodules('mcp') +
    collect_submodules('anyio') +
    collect_submodules('sniffio') +
    ['config', 'decode_image', 'find_all_keys', 'monitor_web']
)

datas = collect_data_files('mcp')

a = Analysis(
    ['../wechat_worker.py'],
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
    name='wechat_worker',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=True,     # 显示控制台（调试）；密钥提取时有进度输出
    uac_admin=True,   # 自动触发 UAC 提权，用于读取进程内存
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=False,
    name='wechat_worker',
)
