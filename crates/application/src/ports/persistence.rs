use rkflash_config::WorkspaceConfig;
use std::path::{Path, PathBuf};

pub trait WorkspaceRepository {
    fn load_workspace(&self, path: &Path) -> Result<WorkspaceConfig, String>;
    fn save_workspace(&self, path: &Path, config: &WorkspaceConfig) -> Result<(), String>;
}

pub trait FileSystem {
    fn read_text(&self, path: &Path) -> Result<String, String>;
    fn list_files(&self, directory: &Path) -> Result<Vec<PathBuf>, String>;
    fn is_file(&self, path: &Path) -> bool;
    fn is_directory(&self, path: &Path) -> bool;
    fn file_size(&self, path: &Path) -> Result<u64, String>;
}
