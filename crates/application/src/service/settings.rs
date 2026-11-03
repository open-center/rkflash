use super::FlashApplication;
use rkflash_api::{SettingsApi, ToolInfo};
use std::path::PathBuf;

impl SettingsApi for FlashApplication {
    fn tool_info(&self) -> ToolInfo {
        self.planner.tool_info()
    }

    fn tool_path(&self) -> Option<PathBuf> {
        self.tool_locator.tool_path()
    }

    fn set_tool_path(&self, path: PathBuf) -> Result<(), String> {
        if !self.file_system.is_file(&path) {
            return Err(format!("工具文件不存在：{}", path.display()));
        }
        self.tool_locator.set_tool_path(Some(path));
        Ok(())
    }

    fn tool_log_directory(&self) -> PathBuf {
        self.operation_log.tool_log_directory()
    }

    fn set_tool_log_directory(&self, path: PathBuf) -> Result<(), String> {
        self.operation_log.set_tool_log_directory(path)
    }
}
