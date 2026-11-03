use super::FlashApplication;
use rkflash_api::{WorkspaceApi, WorkspaceConfig, WORKSPACE_FORMAT_VERSION};
use std::path::Path;

impl WorkspaceApi for FlashApplication {
    fn load_workspace(&self, path: &Path) -> Result<WorkspaceConfig, String> {
        let config = self.workspace_repository.load_workspace(path)?;
        if config.format_version != WORKSPACE_FORMAT_VERSION {
            return Err(format!(
                "不支持的配置版本 {}，当前仅支持版本 {}",
                config.format_version, WORKSPACE_FORMAT_VERSION
            ));
        }
        Ok(config)
    }

    fn save_workspace(&self, path: &Path, config: &WorkspaceConfig) -> Result<(), String> {
        self.workspace_repository.save_workspace(path, config)
    }
}
