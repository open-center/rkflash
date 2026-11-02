//! 本地文件系统适配器；应用层不直接调用 `std::fs`。

use crate::application::FileSystem;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct LocalFileSystem;

impl FileSystem for LocalFileSystem {
    fn read_text(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path)
            .map_err(|error| format!("读取文本失败（{}）：{error}", path.display()))
    }

    fn list_files(&self, directory: &Path) -> Result<Vec<PathBuf>, String> {
        let mut files = fs::read_dir(directory)
            .map_err(|error| format!("读取目录失败（{}）：{error}", directory.display()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        files.sort();
        Ok(files)
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn file_size(&self, path: &Path) -> Result<u64, String> {
        fs::metadata(path)
            .map(|metadata| metadata.len())
            .map_err(|error| format!("读取文件信息失败（{}）：{error}", path.display()))
    }
}
