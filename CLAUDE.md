# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Platform

This tool runs on **Windows only** — it reads WeChat process memory via Win32 APIs (`ctypes.windll`) and accesses Windows-specific file paths (`D:\xwechat_files\...`). Requires Python 3.10+ and admin privileges.

## Commands

```bash
# One-shot entry: auto-detect config, extract keys, then launch
python main.py            # Extract keys + start Web UI (http://localhost:5678)
python main.py decrypt    # Extract keys + decrypt all DBs to decrypted/

# Individual scripts
python find_all_keys.py         # Extract DB keys from WeChat process memory → all_keys.json
python find_image_key.py        # Extract image AES key (single scan)
python find_image_key_monitor.py  # Extract image AES key (continuous monitoring)
python latency_test.py          # Measure message detection latency

# MCP server (requires pip install mcp)
python mcp_server.py            # stdio MCP server for Claude integration
```

## Dependencies

```bash
pip install pycryptodome mcp zstandard
```

- `pycryptodome` — AES decryption
- `mcp` / `fastmcp` — MCP server
- `zstandard` — decompressing WeChat message content (zstd-compressed)

## Architecture

### Key Extraction Flow (`find_all_keys.py`)

Scans WeChat process (`Weixin.exe`) memory for WCDB raw key pattern: `x'<64hex_enc_key><32hex_salt>'`. Each key is validated by HMAC-SHA512 against the corresponding DB file's salt. Saves result to `all_keys.json` as `{rel_path: {enc_key, salt}}`.

### DB Decryption

WeChat uses **SQLCipher 4**: AES-256-CBC, PBKDF2-HMAC-SHA512 (256K iterations), 4096-byte pages with 80-byte reserve (IV=16, HMAC=64). WAL files are pre-allocated fixed size — change detection uses `mtime`, not file size.

Core decrypt functions (`decrypt_page`, `full_decrypt`, `decrypt_wal`) are duplicated in `mcp_server.py` and `monitor_web.py` — no shared module.

### Configuration (`config.py`)

`load_config()` reads `config.json`. On first run, auto-detects `db_dir` from `%APPDATA%\Tencent\xwechat\config\*.ini`. Key fields:
- `db_dir` — path to `...\db_storage` (WeChat's encrypted DB directory)
- `keys_file` — path to `all_keys.json`
- `decrypted_dir` — output for `python main.py decrypt`
- `image_aes_key` — optional V2 image AES key
- `wechat_base_dir` — auto-derived as parent of `db_dir` (for image path resolution)

`config.json` and `all_keys.json` are gitignored (contain sensitive keys).

### Web UI (`monitor_web.py`)

Polls WAL/DB `mtime` every 30ms. On change: full decrypt + WAL patch → query new messages → SSE push to browser. Handles group messages, image inline preview, emoji lookup from `emoticon.db`, and zstd-decompressed message content.

### MCP Server (`mcp_server.py`)

FastMCP (stdio) exposing 7 tools: `get_recent_sessions`, `get_chat_history`, `search_messages`, `get_contacts`, `get_new_messages`, `decode_image`, `get_chat_images`.

`DBCache` class decrypts DBs on demand, caches by `mtime` using fixed filenames in `%TEMP%\wechat_mcp_cache\`. Persists cache metadata across restarts via `_mtimes.json`.

Message table name = `Msg_{md5(username)}` inside `message/message_N.db`.

### Image Decryption (`decode_image.py`)

Three `.dat` formats:
- **Old XOR**: auto-detect key by comparing file header to known image magic bytes
- **V1**: AES-128-ECB + XOR, fixed key `cfcd208495d565ef`
- **V2** (2025-08+): AES-128-ECB + XOR, key extracted from process memory via `find_image_key.py`

Image path: `<wechat_base_dir>\msg\attach\<md5(username)>\<YYYY-MM>\Img\<file_md5>[_t|_h].dat`
MD5 lookup chain: `message_*.db (local_id)` → `message_resource.db (packed_info)` → `.dat` file.

### Database Structure

~26 encrypted DBs under `db_storage/`:
- `session/session.db` — session list with last message summary
- `message/message_*.db` — chat history, one table per contact (`Msg_<md5>`)
- `contact/contact.db` — contacts (`username`, `nick_name`, `remark`)
- `emoticon/emoticon.db` — emoji MD5→CDN mapping
- `media_*/media_*.db` — media file index
