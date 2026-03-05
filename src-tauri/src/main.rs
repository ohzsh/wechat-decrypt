// Prevents console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

// ============ App State ============

struct AppState {
    web_ui_process: Mutex<Option<Child>>,
}

// ============ Path Helpers ============

/// Returns %APPDATA%\WeChatMCP
fn app_data_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(appdata).join("WeChatMCP")
}

/// Returns the directory containing the bundled Python executables.
///
/// Resolution order:
/// 1. `WECHAT_PYTHON_DIR` env var (dev convenience override)
/// 2. Tauri resource dir (production — set by `bundle.resources`)
/// 3. Directory of current exe (fallback)
fn python_exe_dir() -> PathBuf {
    if let Ok(override_dir) = std::env::var("WECHAT_PYTHON_DIR") {
        return PathBuf::from(override_dir);
    }
    // In production, Tauri copies resources next to the exe
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    exe.parent().unwrap_or_else(|| exe.as_path()).to_path_buf()
}

fn worker_exe() -> PathBuf {
    python_exe_dir().join("wechat_worker").join("wechat_worker.exe")
}

fn mcp_server_exe() -> PathBuf {
    python_exe_dir().join("wechat_mcp_server").join("wechat_mcp_server.exe")
}

/// Windows home directory
fn home_dir() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
}

// ============ Tauri Commands ============

#[tauri::command]
fn get_config() -> Result<serde_json::Value, String> {
    let path = app_data_dir().join("config.json");
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(config: serde_json::Value) -> Result<(), String> {
    let dir = app_data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("config.json"), content).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_keys_status() -> serde_json::Value {
    let path = app_data_dir().join("all_keys.json");
    if !path.exists() {
        return serde_json::json!({"exists": false, "count": 0});
    }
    match std::fs::read_to_string(&path).and_then(|s| {
        serde_json::from_str::<serde_json::Value>(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }) {
        Ok(v) => {
            let count = v
                .as_object()
                .map(|m| m.keys().filter(|k| !k.starts_with('_')).count())
                .unwrap_or(0);
            serde_json::json!({"exists": true, "count": count})
        }
        Err(e) => serde_json::json!({"exists": true, "count": 0, "error": e.to_string()}),
    }
}

/// Spawn wechat_worker.exe extract-keys (triggers UAC automatically via embedded manifest).
/// Progress is reported via %TEMP%\wechat_worker_progress.jsonl which we poll and re-emit.
#[tauri::command]
async fn extract_keys(app: AppHandle) -> Result<(), String> {
    let exe = worker_exe();
    if !exe.exists() {
        return Err(format!(
            "Worker executable not found: {}\n请先运行 build\\build_python.bat 编译 Python 组件",
            exe.display()
        ));
    }

    let progress_file = std::env::temp_dir().join("wechat_worker_progress.jsonl");
    // Clear progress file before starting
    let _ = std::fs::write(&progress_file, "");

    let exe_str = exe.to_string_lossy().to_string();
    let progress_str = progress_file.to_string_lossy().to_string();

    tokio::spawn(async move {
        match Command::new(&exe_str).arg("extract-keys").spawn() {
            Ok(mut child) => {
                let mut last_size: u64 = 0;
                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            poll_progress_file(&app, &progress_str, &mut last_size);
                            let _ = app.emit("worker-done", status.success());
                            break;
                        }
                        Ok(None) => {
                            poll_progress_file(&app, &progress_str, &mut last_size);
                            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        }
                        Err(e) => {
                            let _ = app.emit("worker-error", e.to_string());
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let _ = app.emit(
                    "worker-error",
                    format!("无法启动 worker: {e}\n如已启用 UAC 弹窗请点击「是」"),
                );
            }
        }
    });

    Ok(())
}

fn poll_progress_file(app: &AppHandle, path: &str, last_size: &mut u64) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    let current = content.len() as u64;
    if current > *last_size {
        let new_text = &content[*last_size as usize..];
        *last_size = current;
        for line in new_text.lines() {
            if !line.trim().is_empty() {
                let _ = app.emit("worker-progress", line);
            }
        }
    }
}

#[tauri::command]
fn start_web_ui(state: State<'_, AppState>) -> Result<(), String> {
    let mut proc = state.web_ui_process.lock().unwrap();
    if proc.is_some() {
        return Ok(());
    }
    let exe = worker_exe();
    if !exe.exists() {
        return Err(format!("Worker executable not found: {}", exe.display()));
    }
    match Command::new(&exe).arg("web-ui").spawn() {
        Ok(child) => {
            *proc = Some(child);
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn stop_web_ui(state: State<'_, AppState>) -> Result<(), String> {
    let mut proc = state.web_ui_process.lock().unwrap();
    if let Some(mut child) = proc.take() {
        let _ = child.kill();
    }
    Ok(())
}

#[tauri::command]
fn get_web_ui_status(state: State<'_, AppState>) -> bool {
    let mut proc = state.web_ui_process.lock().unwrap();
    if let Some(child) = proc.as_mut() {
        match child.try_wait() {
            Ok(None) => true,
            _ => {
                *proc = None;
                false
            }
        }
    } else {
        false
    }
}

#[tauri::command]
fn register_claude_desktop() -> Result<String, String> {
    let appdata = std::env::var("APPDATA").map_err(|e| e.to_string())?;
    let config_path = PathBuf::from(&appdata)
        .join("Claude")
        .join("claude_desktop_config.json");

    let exe = mcp_server_exe();
    if !exe.exists() {
        return Err(format!(
            "MCP server executable not found: {}\n请先运行 build\\build_python.bat",
            exe.display()
        ));
    }

    let mut cfg: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if cfg.get("mcpServers").is_none() {
        cfg["mcpServers"] = serde_json::json!({});
    }
    cfg["mcpServers"]["wechat"] = serde_json::json!({
        "command": exe.to_string_lossy(),
        "args": []
    });

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(&config_path, content).map_err(|e| e.to_string())?;

    Ok(format!("已注册到 Claude Desktop: {}", config_path.display()))
}

#[tauri::command]
fn register_claude_code() -> Result<String, String> {
    let home = home_dir().ok_or("无法找到用户主目录")?;
    let config_path = home.join(".claude.json");

    let exe = mcp_server_exe();
    if !exe.exists() {
        return Err(format!(
            "MCP server executable not found: {}\n请先运行 build\\build_python.bat",
            exe.display()
        ));
    }

    let mut cfg: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if cfg.get("mcpServers").is_none() {
        cfg["mcpServers"] = serde_json::json!({});
    }
    cfg["mcpServers"]["wechat"] = serde_json::json!({
        "command": exe.to_string_lossy(),
        "args": [],
        "type": "stdio"
    });

    let content = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(&config_path, content).map_err(|e| e.to_string())?;

    Ok(format!("已注册到 Claude Code: {}", config_path.display()))
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.to_string_lossy().to_string();
    let reg_key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    if enabled {
        Command::new("reg")
            .args(["add", reg_key, "/v", "WeChatMCP", "/t", "REG_SZ", "/d", &exe_str, "/f"])
            .status()
            .map_err(|e| e.to_string())?;
    } else {
        Command::new("reg")
            .args(["delete", reg_key, "/v", "WeChatMCP", "/f"])
            .status()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_autostart_status() -> bool {
    Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "WeChatMCP",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ============ Main ============

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            web_ui_process: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_keys_status,
            extract_keys,
            start_web_ui,
            stop_web_ui,
            get_web_ui_status,
            register_claude_desktop,
            register_claude_code,
            set_autostart,
            get_autostart_status,
        ])
        .on_window_event(|window, event| {
            // Kill web UI process when main window is destroyed
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.try_state::<AppState>() {
                    let mut proc = state.web_ui_process.lock().unwrap();
                    if let Some(mut child) = proc.take() {
                        let _ = child.kill();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run()
}
