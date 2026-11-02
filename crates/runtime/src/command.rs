//! 外部工具进程的启动配置与仅用于日志展示的参数格式化。

use crate::host::{OsHost, ToolProcessContext};
use std::ffi::{OsStr, OsString};
use std::process::Command;

/// 设置所有平台共用的环境（隔离 HOME），再交给平台实现设置目录、进程组与窗口属性。
pub(super) fn configure_tool_process(
    host: &dyn OsHost,
    command: &mut Command,
    context: &ToolProcessContext<'_>,
) {
    command.env("HOME", context.tool_home_directory);
    host.configure_tool_process(command, context);
}

pub(super) fn format_command(tool: &std::path::Path, args: &[OsString]) -> String {
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
