//! 外部工具进程环境和仅用于日志展示的参数格式化。

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Child, Command};

/// Windows 上 rkdeveloptool 读取该变量决定内部日志目录（替代 Unix 的 HOME 软链接映射）。
#[cfg(windows)]
pub(super) const TOOL_LOG_DIRECTORY_ENV: &str = "RKDEVELOPTOOL_LOG_DIR";

/// `tool_log_directory` 只在 Windows 上使用；Unix 通过 HOME 下的软链接映射日志目录。
pub(super) fn configure_tool_process(
    command: &mut Command,
    tool_path: &Path,
    tool_home_directory: &Path,
    tool_log_directory: Option<&Path>,
) {
    #[cfg(not(windows))]
    {
        let _ = tool_log_directory;
        if let Some(directory) = tool_path.parent() {
            command.current_dir(directory);
        }
    }
    // Windows 的安装目录（Program Files）对普通用户只读，工具在工作目录下写
    // log/ 与读取 config.ini，因此改用可写的运行时隔离目录。
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let _ = tool_path;
        let _ = std::fs::create_dir_all(tool_home_directory);
        command.current_dir(tool_home_directory);
        if let Some(directory) = tool_log_directory {
            command.env(TOOL_LOG_DIRECTORY_ENV, directory);
        }
        // GUI 进程启动控制台工具时不弹出黑色命令行窗口。
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.env("HOME", tool_home_directory);
    // 让工具进程成为独立进程组的组长：终止时可以按组清理它派生的孙进程，
    // 避免残留进程继续占用 USB 设备或持有输出管道。
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
}

/// 终止工具进程及其进程组；退化为仅终止直接子进程的兜底保持各平台可用。
pub(super) fn terminate_tool_process(child: &mut Child) {
    #[cfg(unix)]
    {
        let process_group_id = child.id() as libc::c_int;
        // SAFETY: `killpg` 只对已验证的整数进程组号发送信号。
        if unsafe { libc::killpg(process_group_id, libc::SIGKILL) } == 0 {
            return;
        }
    }
    let _ = child.kill();
}

pub(super) fn format_command(tool: &Path, args: &[OsString]) -> String {
    let mut parts = vec![shell_display(tool.as_os_str())];
    parts.extend(args.iter().map(|argument| shell_display(argument)));
    parts.join(" ")
}

fn shell_display(value: &OsStr) -> String {
    let text = value.to_string_lossy();
    if text.contains(char::is_whitespace) || text.contains('"') {
        format!("\"{}\"", text.replace('"', "\\\""))
    } else {
        text.into_owned()
    }
}
