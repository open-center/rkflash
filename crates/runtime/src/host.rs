//! 运行时依赖的操作系统能力。通用运行时只通过本接口访问平台差异，
//! 具体实现位于 `crates/platforms/*`（Unix：进程组、localtime_r、软链接；Windows：Win32 API）。

use std::path::{Path, PathBuf};
use std::process::{Child, Command};

/// 启动外部烧录工具时的目录约定。
pub struct ToolProcessContext<'a> {
    pub tool_path: &'a Path,
    /// 每次运行独立的隔离目录，同时作为工具的 HOME。
    pub tool_home_directory: &'a Path,
    /// 用户选择的工具日志目录；设备轮询等不需要日志的调用为 `None`。
    pub tool_log_directory: Option<&'a Path>,
}

/// 本地时间，用于操作日志文件名与日志行时间戳。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub hour: i32,
    pub minute: i32,
    pub second: i32,
    pub millis: u32,
}

pub trait OsHost: Send + Sync {
    /// 设置工具进程的工作目录、环境变量、进程组或窗口属性（HOME 已由运行时设置）。
    fn configure_tool_process(&self, command: &mut Command, context: &ToolProcessContext<'_>);

    /// 终止工具进程及其派生进程。
    fn terminate_tool_process(&self, child: &mut Child);

    /// 判断进程号对应的进程是否已经退出；无法判断时返回 `false`（保守地保留目录）。
    fn process_has_exited(&self, pid: &str) -> bool;

    fn local_time(&self) -> Option<LocalTime>;

    /// 让外部工具把内部日志写入 `log_directory`，返回实际使用的日志目录。
    /// `tool_log_relative_path` 是工具在 HOME 下写日志的固定相对路径。
    fn prepare_log_redirect(
        &self,
        tool_home_directory: &Path,
        tool_log_relative_path: &Path,
        log_directory: &Path,
    ) -> Result<PathBuf, String>;
}
