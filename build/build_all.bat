@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul

echo ============================================================
echo   WeChat MCP - 完整构建（Python + Tauri）
echo ============================================================
echo.

cd /d "%~dp0.."

:: ---- Step 1: 编译 Python 组件 ----
echo [步骤 1/2] 编译 Python 组件...
call build\build_python.bat
if errorlevel 1 exit /b 1

:: ---- Step 2: Tauri 构建 ----
echo.
echo [步骤 2/2] 构建 Tauri 桌面应用...

:: 检查 Rust / Cargo
cargo --version >nul 2>&1 || (
    echo [!] 未找到 cargo，请安装 Rust: https://rustup.rs
    pause
    exit /b 1
)

:: 检查 tauri-cli
cargo tauri --version >nul 2>&1 || (
    echo [*] 安装 tauri-cli...
    cargo install tauri-cli --version "^2.0" --locked
)

:: 生成图标（如果 assets/icon.png 存在）
if exist assets\icon.png (
    echo [*] 生成应用图标...
    cargo tauri icon assets\icon.png
) else (
    echo [*] 生成占位图标...
    python generate_icons.py && cargo tauri icon assets\icon.png
)

:: 设置 Python exe 目录供 Tauri 使用
set WECHAT_PYTHON_DIR=%CD%\build\dist

cargo tauri build
if errorlevel 1 (
    echo [!] Tauri 构建失败
    pause
    exit /b 1
)

echo.
echo ============================================================
echo   构建完成！
echo   安装包位置: src-tauri\target\release\bundle\
echo ============================================================
pause
