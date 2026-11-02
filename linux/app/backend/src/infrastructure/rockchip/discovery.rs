//! Rockchip 官方工具的受控发现顺序。

use std::env;
use std::path::PathBuf;

const TOOL_FILE_NAME: &str = "upgrade_tool";

pub(crate) fn discover_upgrade_tool() -> Option<PathBuf> {
    if let Some(path) = env::var_os("RK_UPGRADE_TOOL").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }

    let mut candidates = Vec::new();
    if let Ok(executable) = env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("tools").join(TOOL_FILE_NAME));
            candidates.push(directory.join(TOOL_FILE_NAME));
        }
    }
    if let Ok(directory) = env::current_dir() {
        candidates.push(directory.join("tools").join(TOOL_FILE_NAME));
        candidates.push(directory.join(TOOL_FILE_NAME));
    }
    candidates.push(PathBuf::from("/usr/lib/rkflash/upgrade_tool"));
    if let Some(path) = env::var_os("PATH") {
        candidates.extend(env::split_paths(&path).map(|directory| directory.join(TOOL_FILE_NAME)));
    }
    candidates.into_iter().find(|path| path.is_file())
}
