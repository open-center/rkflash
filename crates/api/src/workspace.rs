//! 工作区配置：下载列表与各页路径的导入导出。

pub use rkflash_config::{ImageEntry, WorkspaceConfig, WORKSPACE_FORMAT_VERSION};
use std::path::Path;

pub trait WorkspaceApi {
    fn load_workspace(&self, path: &Path) -> Result<WorkspaceConfig, String>;
    fn save_workspace(&self, path: &Path, config: &WorkspaceConfig) -> Result<(), String>;
}
