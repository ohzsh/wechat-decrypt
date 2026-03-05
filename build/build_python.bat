@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul

echo ============================================================
echo   WeChat MCP - Python 组件编译
echo ============================================================
echo.

:: 切换到项目根目录（build/ 的上级）
cd /d "%~dp0.."
echo [*] 工作目录: %CD%
echo.

:: 检查 Python
python --version >nul 2>&1 || (
    echo [!] 未找到 Python，请安装 Python 3.10+
    pause
    exit /b 1
)

:: 安装依赖
echo [1/3] 安装 Python 依赖...
pip install -r requirements.txt --quiet
if errorlevel 1 (
    echo [!] 依赖安装失败
    pause
    exit /b 1
)
echo     OK
echo.

:: 编译 MCP Server
echo [2/3] 编译 wechat_mcp_server.exe ...
pyinstaller build\build_server.spec ^
    --distpath build\dist ^
    --workpath build\work ^
    --clean ^
    --noconfirm
if errorlevel 1 (
    echo [!] wechat_mcp_server 编译失败
    pause
    exit /b 1
)
echo     OK: build\dist\wechat_mcp_server\
echo.

:: 编译 Worker
echo [3/3] 编译 wechat_worker.exe ...
pyinstaller build\build_worker.spec ^
    --distpath build\dist ^
    --workpath build\work ^
    --clean ^
    --noconfirm
if errorlevel 1 (
    echo [!] wechat_worker 编译失败
    pause
    exit /b 1
)
echo     OK: build\dist\wechat_worker\
echo.

echo ============================================================
echo   编译完成！
echo   - build\dist\wechat_mcp_server\wechat_mcp_server.exe
echo   - build\dist\wechat_worker\wechat_worker.exe
echo.
echo   下一步: 运行 build\build_all.bat 生成完整安装包
echo ============================================================
pause
