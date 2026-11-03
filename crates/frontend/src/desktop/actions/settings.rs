//! 烧录工具与工具日志目录 ↔ `SettingsApi`。

use super::super::*;

impl RkFlashApp {
    /// 启动时读取工具能力、工具路径与日志目录。
    pub(in crate::desktop) fn load_tool_settings(&mut self) {
        self.tool = self.application.tool_info();
        self.tool_log_directory = self.application.tool_log_directory().display().to_string();
        match self.application.tool_path() {
            Some(path) => {
                self.push_log(LogLevel::Success, format!("已找到工具：{}", path.display()))
            }
            None => self.push_log(
                LogLevel::Warning,
                format!("未找到 {}，请从右上角选择工具", self.tool.name),
            ),
        }
    }

    pub(in crate::desktop) fn tool_path(&self) -> Option<PathBuf> {
        self.application.tool_path()
    }

    pub(in crate::desktop) fn pick_tool(&mut self) {
        self.spawn_file_dialog(
            FileDialogSpec::pick_file(&format!("选择 {}", self.tool.name), "可执行文件", &[]),
            FileDialogTarget::ToolExecutable,
        );
    }

    pub(in crate::desktop) fn set_tool_path(&mut self, path: PathBuf) {
        match self.application.set_tool_path(path.clone()) {
            Ok(()) => self.push_log(
                LogLevel::Success,
                format!("已设置工具路径：{}", path.display()),
            ),
            Err(error) => self.push_log(LogLevel::Error, error),
        }
    }

    pub(in crate::desktop) fn set_tool_log_directory(&mut self, path: PathBuf) {
        match self.application.set_tool_log_directory(path) {
            Ok(()) => {
                let active_path = self.application.tool_log_directory();
                self.tool_log_directory = active_path.display().to_string();
                self.push_log(
                    LogLevel::Success,
                    format!("工具日志目录已切换至：{}", active_path.display()),
                );
            }
            Err(error) => self.report_error("无法切换工具日志目录", error),
        }
    }
}
