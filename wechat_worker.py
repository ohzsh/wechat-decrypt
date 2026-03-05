"""
WeChat Worker - 供 Tauri GUI 调用的统一 CLI 工作进程

用法:
  wechat_worker.exe extract-keys   提取数据库密钥（需要管理员权限）
  wechat_worker.exe web-ui         启动 Web UI 监听器（阻塞）
  wechat_worker.exe check-config   检查配置并返回 JSON 状态

进度通过两条通道回报：
  1. stdout  (JSON 行，供非提权测试)
  2. %TEMP%\wechat_worker_progress.jsonl  (供 Tauri 轮询，提权后 stdio 断开)
"""
import sys
import os
import json
import time
import io

PROGRESS_FILE = os.path.join(os.environ.get('TEMP', os.getcwd()), 'wechat_worker_progress.jsonl')


# ---- 进度输出 ----

class _ProgressTee(io.TextIOBase):
    """将 stdout 写入同时追加到进度文件（按行）。"""

    def __init__(self, orig):
        self._orig = orig
        self._buf = ''

    def write(self, s: str) -> int:
        self._orig.write(s)
        self._buf += s
        while '\n' in self._buf:
            line, self._buf = self._buf.split('\n', 1)
            if line.strip():
                _append_progress('log', line)
        return len(s)

    def flush(self):
        self._orig.flush()


def _append_progress(type_: str, message: str, **extra):
    entry = {'type': type_, 'message': message, 'ts': time.time()}
    entry.update(extra)
    line = json.dumps(entry, ensure_ascii=False)
    try:
        with open(PROGRESS_FILE, 'a', encoding='utf-8') as f:
            f.write(line + '\n')
    except OSError:
        pass


def _emit(type_: str, message: str, **extra):
    _append_progress(type_, message, **extra)
    entry = {'type': type_, 'message': message, 'ts': time.time()}
    entry.update(extra)
    print(json.dumps(entry, ensure_ascii=False), flush=True)


# ---- 子命令实现 ----

def cmd_extract_keys():
    """提取微信数据库密钥（需要管理员权限）。"""
    # 清空进度文件
    try:
        open(PROGRESS_FILE, 'w').close()
    except OSError:
        pass

    orig_stdout = sys.stdout
    sys.stdout = _ProgressTee(orig_stdout)

    try:
        _emit('progress', '加载配置...')
        # find_all_keys 在模块层加载 config，frozen 环境下自动使用 %APPDATA%\WeChatMCP\
        import find_all_keys  # noqa: F401 — 触发模块级初始化
        _emit('progress', '开始扫描微信进程内存（需要管理员权限）...')
        find_all_keys.main()
        _emit('done', '密钥提取完成', success=True)
    except Exception as e:
        _emit('error', str(e), success=False)
        sys.stdout = orig_stdout
        sys.exit(1)
    finally:
        sys.stdout = orig_stdout


def cmd_web_ui():
    """启动 Web UI 监听器（阻塞直到进程退出）。"""
    from monitor_web import main as web_main
    web_main()


def cmd_check_config():
    """检查配置和密钥状态，输出 JSON 到 stdout。"""
    result = {}
    try:
        from config import load_config, CONFIG_FILE
        cfg = load_config()
        result['config_file'] = CONFIG_FILE
        result['db_dir'] = cfg.get('db_dir', '')
        result['db_dir_exists'] = os.path.isdir(cfg.get('db_dir', ''))

        keys_file = cfg.get('keys_file', '')
        result['keys_file'] = keys_file
        if os.path.exists(keys_file):
            with open(keys_file) as f:
                keys = json.load(f)
            count = sum(1 for k in keys if not k.startswith('_'))
            result['keys_count'] = count
        else:
            result['keys_count'] = 0

        result['ok'] = True
    except Exception as e:
        result['ok'] = False
        result['error'] = str(e)

    print(json.dumps(result, ensure_ascii=False))


# ---- 入口 ----

def main():
    if len(sys.argv) < 2:
        print('Usage: wechat_worker.exe <extract-keys|web-ui|check-config>', file=sys.stderr)
        sys.exit(1)

    cmd = sys.argv[1]
    if cmd == 'extract-keys':
        cmd_extract_keys()
    elif cmd == 'web-ui':
        cmd_web_ui()
    elif cmd == 'check-config':
        cmd_check_config()
    else:
        print(f'Unknown command: {cmd}', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
