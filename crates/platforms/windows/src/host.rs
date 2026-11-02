//! Windows 的进程与日志实现：无控制台窗口启动、Win32 进程查询与本地时间。
//!
//! Windows 创建软链接需要管理员或开发者模式，因此不做 HOME 映射：
//! 日志目录在启动工具时通过 `RKDEVELOPTOOL_LOG_DIR` 直接传给 rkdeveloptool。

use rkflash_runtime::{LocalTime, OsHost, ToolProcessContext};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_INVALID_PARAMETER};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// rkdeveloptool（本仓库 native/ 中的修改版）读取该变量决定内部日志目录。
const TOOL_LOG_DIRECTORY_ENV: &str = "RKDEVELOPTOOL_LOG_DIR";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const STILL_ACTIVE: u32 = 259;

#[derive(Default)]
pub struct WindowsHost;

impl OsHost for WindowsHost {
    fn configure_tool_process(&self, command: &mut Command, context: &ToolProcessContext<'_>) {
        // 安装目录（Program Files）对普通用户只读，工具在工作目录下写 log/ 与读取
        // config.ini，因此改用可写的运行时隔离目录。
        let _ = fs::create_dir_all(context.tool_home_directory);
        command.current_dir(context.tool_home_directory);
        if let Some(directory) = context.tool_log_directory {
            command.env(TOOL_LOG_DIRECTORY_ENV, directory);
        }
        // GUI 进程启动控制台工具时不弹出黑色命令行窗口。
        command.creation_flags(CREATE_NO_WINDOW);
    }

    fn terminate_tool_process(&self, child: &mut Child) {
        let _ = child.kill();
    }

    fn process_has_exited(&self, pid: &str) -> bool {
        let Ok(pid) = pid.parse::<u32>() else {
            return false;
        };
        if pid == 0 {
            return false;
        }
        // SAFETY: 只以查询权限打开进程句柄，读取退出码后立即关闭。
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                // 进程号不存在时返回 ERROR_INVALID_PARAMETER；拒绝访问说明进程仍存在。
                return GetLastError() == ERROR_INVALID_PARAMETER;
            }
            let mut exit_code = 0u32;
            let queried = GetExitCodeProcess(handle, &mut exit_code) != 0;
            CloseHandle(handle);
            queried && exit_code != STILL_ACTIVE
        }
    }

    fn local_time(&self) -> Option<LocalTime> {
        // SAFETY: GetLocalTime 只写入调用方提供的 SYSTEMTIME 结构。
        let local = unsafe {
            let mut local = std::mem::zeroed();
            GetLocalTime(&mut local);
            local
        };
        Some(LocalTime {
            year: i32::from(local.wYear),
            month: i32::from(local.wMonth),
            day: i32::from(local.wDay),
            hour: i32::from(local.wHour),
            minute: i32::from(local.wMinute),
            second: i32::from(local.wSecond),
            millis: u32::from(local.wMilliseconds),
        })
    }

    fn prepare_log_redirect(
        &self,
        _tool_home_directory: &Path,
        _tool_log_relative_path: &Path,
        log_directory: &Path,
    ) -> Result<PathBuf, String> {
        fs::create_dir_all(log_directory).map_err(|error| {
            format!("无法创建工具日志目录 {}：{error}", log_directory.display())
        })?;
        std::path::absolute(log_directory)
            .map_err(|error| format!("无法解析工具日志目录 {}：{error}", log_directory.display()))
    }
}
