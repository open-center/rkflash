//! 前端调用后端的唯一能力端口。

use crate::{ApplicationUpdate, FlashOperation, ImageEntry, WorkspaceConfig};
use std::path::{Path, PathBuf};

pub trait FlashService {
    fn poll(&mut self) -> ApplicationUpdate;
    fn execute(&self, operation: FlashOperation) -> Result<(), String>;
    fn validate(&self, operation: &FlashOperation) -> Result<(), String>;
    /// 当前平台的烧录工具是否支持该操作；不支持时前端禁用对应入口。
    fn supports(&self, operation: &FlashOperation) -> bool;
    /// 当前平台使用的外部烧录工具名称（upgrade_tool 或 rkdeveloptool）。
    fn tool_name(&self) -> &'static str;
    fn cancel(&self);
    fn is_active(&self) -> bool;
    fn tool_path(&self) -> Option<PathBuf>;
    fn set_tool_path(&self, path: PathBuf) -> Result<(), String>;
    fn tool_log_directory(&self) -> PathBuf;
    fn set_tool_log_directory(&self, path: PathBuf) -> Result<(), String>;
    fn operation_log_path(&self) -> Option<PathBuf>;
    fn append_operation_log(&self, level: &str, text: &str) -> Result<(), String>;
    fn default_images(&self) -> Vec<ImageEntry>;
    fn file_size(&self, path: &Path) -> Result<u64, String>;
    fn import_parameter(&self, path: &Path) -> Result<Vec<ImageEntry>, String>;
    fn load_workspace(&self, path: &Path) -> Result<WorkspaceConfig, String>;
    fn save_workspace(&self, path: &Path, config: &WorkspaceConfig) -> Result<(), String>;
}
