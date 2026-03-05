'use strict';

// Tauri 2.x injected globals (requires withGlobalTauri: true in tauri.conf.json)
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const App = (() => {
  // ---- Tab switching ----
  function initTabs() {
    document.querySelectorAll('.nav-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const tab = btn.dataset.tab;
        document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
        document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
        btn.classList.add('active');
        document.getElementById(`tab-${tab}`)?.classList.add('active');
      });
    });
  }

  // ---- Logging ----
  function log(msg, cls = '') {
    const el = document.getElementById('app-log');
    const line = document.createElement('div');
    line.className = `log-line ${cls}`;
    line.textContent = `[${new Date().toLocaleTimeString()}] ${msg}`;
    el.appendChild(line);
    el.scrollTop = el.scrollHeight;
    // Also truncate if too long
    while (el.children.length > 500) el.removeChild(el.firstChild);
  }

  function appendKeysLog(msg, cls = '') {
    const el = document.getElementById('keys-log');
    if (!el) return;
    el.textContent += msg + '\n';
    el.scrollTop = el.scrollHeight;
  }

  function clearLog() {
    document.getElementById('app-log').innerHTML = '';
  }

  function setMsg(id, msg, isErr = false) {
    const el = document.getElementById(id);
    if (!el) return;
    el.textContent = msg;
    el.className = `status-msg ${isErr ? 'err' : 'ok'}`;
  }

  // ---- Config ----
  async function loadConfig() {
    try {
      const cfg = await invoke('get_config');
      document.getElementById('cfg-db-dir').value = cfg.db_dir || '';
      document.getElementById('cfg-keys-file').value = cfg.keys_file || '（自动）';
      document.getElementById('cfg-decrypted-dir').value = cfg.decrypted_dir || '（自动）';
      document.getElementById('cfg-image-dir').value = cfg.decoded_image_dir || '（自动）';
      log('配置已加载', 'info');
    } catch (e) {
      log(`加载配置失败: ${e}`, 'err');
    }
  }

  async function saveConfig() {
    const dbDir = document.getElementById('cfg-db-dir').value.trim();
    if (!dbDir) {
      setMsg('config-msg', 'DB 目录不能为空', true);
      return;
    }
    try {
      const existing = await invoke('get_config');
      const updated = { ...existing, db_dir: dbDir };
      await invoke('save_config', { config: updated });
      setMsg('config-msg', '配置已保存');
      log('配置已保存');
      await loadConfig();
    } catch (e) {
      setMsg('config-msg', `保存失败: ${e}`, true);
    }
  }

  async function browseDbDir() {
    // Tauri file dialog is not available without the dialog plugin.
    // Prompt user to type manually.
    const cur = document.getElementById('cfg-db-dir').value;
    const val = window.prompt('请输入 db_storage 目录路径:', cur);
    if (val !== null) document.getElementById('cfg-db-dir').value = val;
  }

  // ---- Keys ----
  async function refreshKeysStatus() {
    try {
      const status = await invoke('get_keys_status');
      const card = document.getElementById('keys-status-card');
      const icon = document.getElementById('keys-status-icon');
      const title = document.getElementById('keys-status-title');
      const sub = document.getElementById('keys-status-sub');
      card.className = 'status-card';
      if (status.exists && status.count > 0) {
        card.classList.add('ok');
        icon.textContent = '\u2705';
        title.textContent = `${status.count} 个数据库密钥`;
        sub.textContent = '密钥文件正常，可以查询消息';
      } else if (status.exists) {
        card.classList.add('err');
        icon.textContent = '\u26A0\uFE0F';
        title.textContent = '密钥文件异常';
        sub.textContent = status.error || '文件存在但无法读取';
      } else {
        icon.textContent = '\u274C';
        title.textContent = '尚未提取密钥';
        sub.textContent = '请点击下方按钮提取密钥（需要管理员权限）';
      }
    } catch (e) {
      log(`刷新密钥状态失败: ${e}`, 'err');
    }
  }

  async function extractKeys() {
    const btn = document.getElementById('btn-extract');
    btn.disabled = true;
    btn.textContent = '提取中...';
    document.getElementById('keys-log').textContent = '';
    log('开始提取密钥（将弹出 UAC 确认框，请点击「是」）...', 'info');

    try {
      await invoke('extract_keys');
    } catch (e) {
      log(`启动提取失败: ${e}`, 'err');
      btn.disabled = false;
      btn.textContent = '提取 / 刷新密钥';
    }
  }

  // ---- Web UI ----
  let webUiPollTimer = null;

  async function startWebUi() {
    try {
      await invoke('start_web_ui');
      log('Web UI 已启动: http://localhost:5678', 'info');
      setMsg('webui-msg', 'Web UI 已启动');
      updateWebUiStatus(true);
      // Poll status every 5s
      webUiPollTimer = setInterval(pollWebUiStatus, 5000);
    } catch (e) {
      setMsg('webui-msg', `启动失败: ${e}`, true);
      log(`Web UI 启动失败: ${e}`, 'err');
    }
  }

  async function stopWebUi() {
    try {
      await invoke('stop_web_ui');
      if (webUiPollTimer) { clearInterval(webUiPollTimer); webUiPollTimer = null; }
      log('Web UI 已停止');
      setMsg('webui-msg', 'Web UI 已停止');
      updateWebUiStatus(false);
    } catch (e) {
      log(`停止 Web UI 失败: ${e}`, 'err');
    }
  }

  async function pollWebUiStatus() {
    try {
      const running = await invoke('get_web_ui_status');
      updateWebUiStatus(running);
      if (!running && webUiPollTimer) {
        clearInterval(webUiPollTimer);
        webUiPollTimer = null;
      }
    } catch (_) {}
  }

  function updateWebUiStatus(running) {
    const card = document.getElementById('webui-status-card');
    const icon = document.getElementById('webui-status-icon');
    const title = document.getElementById('webui-status-title');
    const sub = document.querySelector('#webui-status-card .status-card-sub');

    document.getElementById('btn-start-webui').disabled = running;
    document.getElementById('btn-stop-webui').disabled = !running;
    document.getElementById('btn-open-browser').disabled = !running;

    if (running) {
      card.className = 'status-card running';
      icon.textContent = '\u25B6\uFE0F';
      title.textContent = '运行中';
      sub.textContent = 'http://localhost:5678';
    } else {
      card.className = 'status-card';
      icon.textContent = '\u23F9';
      title.textContent = '未运行';
      sub.textContent = '点击启动开始监听';
    }
  }

  function openWebUi() {
    window.open('http://localhost:5678', '_blank');
  }

  // ---- Claude Registration ----
  async function registerClaudeDesktop() {
    try {
      const msg = await invoke('register_claude_desktop');
      setMsg('desktop-msg', msg);
      log(msg, 'ok');
    } catch (e) {
      setMsg('desktop-msg', String(e), true);
      log(`注册 Claude Desktop 失败: ${e}`, 'err');
    }
  }

  async function registerClaudeCode() {
    try {
      const msg = await invoke('register_claude_code');
      setMsg('code-msg', msg);
      log(msg, 'ok');
    } catch (e) {
      setMsg('code-msg', String(e), true);
      log(`注册 Claude Code 失败: ${e}`, 'err');
    }
  }

  // ---- Settings ----
  async function loadSettings() {
    try {
      const on = await invoke('get_autostart_status');
      const toggle = document.getElementById('autostart-toggle');
      const label = document.getElementById('autostart-label');
      toggle.checked = on;
      label.textContent = on ? '已启用' : '已禁用';
    } catch (_) {}
  }

  async function setAutostart(enabled) {
    try {
      await invoke('set_autostart', { enabled });
      document.getElementById('autostart-label').textContent = enabled ? '已启用' : '已禁用';
      log(`开机自启: ${enabled ? '启用' : '禁用'}`, 'info');
    } catch (e) {
      log(`设置开机自启失败: ${e}`, 'err');
    }
  }

  // ---- Event listeners ----
  function initEvents() {
    listen('worker-progress', event => {
      try {
        const data = typeof event.payload === 'string'
          ? JSON.parse(event.payload)
          : event.payload;
        const msg = data.message || String(event.payload);
        appendKeysLog(msg);
        log(msg);
      } catch (_) {
        appendKeysLog(String(event.payload));
        log(String(event.payload));
      }
    });

    listen('worker-done', event => {
      const success = event.payload;
      const btn = document.getElementById('btn-extract');
      btn.disabled = false;
      btn.textContent = '提取 / 刷新密钥';
      if (success) {
        log('密钥提取成功！', 'ok');
        appendKeysLog('\n[完成] 密钥提取成功');
        refreshKeysStatus();
      } else {
        log('密钥提取失败，请查看日志', 'err');
        appendKeysLog('\n[错误] 提取失败，请确认微信已登录并以管理员权限运行');
      }
    });

    listen('worker-error', event => {
      const btn = document.getElementById('btn-extract');
      btn.disabled = false;
      btn.textContent = '提取 / 刷新密钥';
      const msg = String(event.payload);
      log(`Worker 错误: ${msg}`, 'err');
      appendKeysLog(`\n[错误] ${msg}`);
    });
  }

  // ---- Init ----
  async function init() {
    initTabs();
    initEvents();
    await loadConfig();
    await refreshKeysStatus();
    await loadSettings();
    await pollWebUiStatus();

    // Show MCP server path
    const appdata = await invoke('get_config').then(c => c).catch(() => ({}));
    // We display the expected path (computed in Rust)
    document.getElementById('mcp-server-path').textContent =
      '<安装目录>\\wechat_mcp_server\\wechat_mcp_server.exe';

    log('WeChat MCP Manager 已启动', 'info');
  }

  document.addEventListener('DOMContentLoaded', init);

  return { loadConfig, saveConfig, browseDbDir, extractKeys, startWebUi, stopWebUi, openWebUi, registerClaudeDesktop, registerClaudeCode, setAutostart, clearLog };
})();
