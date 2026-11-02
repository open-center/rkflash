//! 外部烧录工具的受控发现顺序；工具名与系统安装路径由各平台传入。

use std::env;
use std::path::{Path, PathBuf};

/// 依次查找：环境变量 → 程序目录（含 `tools/`）→ 当前目录（含 `tools/`）→ 系统路径 → `PATH`。
pub fn discover_tool(
    environment: &str,
    file_name: &str,
    system_paths: &[&Path],
) -> Option<PathBuf> {
    if let Some(path) = env::var_os(environment).map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }

    let mut candidates = Vec::new();
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("tools").join(file_name));
            candidates.push(directory.join(file_name));
        }
    }
    if let Ok(directory) = env::current_dir() {
        candidates.push(directory.join("tools").join(file_name));
        candidates.push(directory.join(file_name));
    }
    candidates.extend(system_paths.iter().map(|path| path.to_path_buf()));
    if let Some(path) = env::var_os("PATH") {
        candidates.extend(env::split_paths(&path).map(|directory| directory.join(file_name)));
    }
    candidates.into_iter().find(|path| path.is_file())
}
