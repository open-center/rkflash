//! 烧录工具设置：工具能力、工具路径与工具日志目录。

use crate::{DeviceControl, DeviceQuery, StorageType};
use std::path::PathBuf;

/// 当前平台烧录工具的名称与能力；前端据此显示或禁用对应入口，不再按工具名判断。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolInfo {
    /// 界面中显示的工具名称（upgrade_tool 或 rkdeveloptool）。
    pub name: &'static str,
    pub queries: Vec<DeviceQuery>,
    pub controls: Vec<DeviceControl>,
    pub loader_storages: Vec<StorageType>,
    /// 整包升级前可选的目标存储；为空表示工具自动识别，不显示该选项。
    pub firmware_storages: Vec<StorageType>,
    pub serial_log: bool,
}

pub trait SettingsApi {
    fn tool_info(&self) -> ToolInfo;
    fn tool_path(&self) -> Option<PathBuf>;
    fn set_tool_path(&self, path: PathBuf) -> Result<(), String>;
    fn tool_log_directory(&self) -> PathBuf;
    fn set_tool_log_directory(&self, path: PathBuf) -> Result<(), String>;
}
