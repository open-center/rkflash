//! 外部工具的受控发现顺序。

use std::env;
use std::path::PathBuf;

const UPGRADE_TOOL_FILE_NAME: &str = "upgrade_tool";

pub(crate) fn discover_upgrade_tool() -> Option<PathBuf> {
    discover(
        "RK_UPGRADE_TOOL",
        UPGRADE_TOOL_FILE_NAME,
        &[PathBuf::from("/usr/lib/rkflash/upgrade_tool")],
    )
}

/// rkdeveloptool 随 Windows 安装包放在 rkflash.exe 同目录。
pub(crate) fn discover_rkdeveloptool() -> Option<PathBuf> {
    discover(
        "RK_DEVELOP_TOOL",
        &format!("rkdeveloptool{}", env::consts::EXE_SUFFIX),
        &[],
    )
}

fn discover(environment: &str, file_name: &str, system_paths: &[PathBuf]) -> Option<PathBuf> {
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
    candidates.extend_from_slice(system_paths);
    if let Some(path) = env::var_os("PATH") {
        candidates.extend(env::split_paths(&path).map(|directory| directory.join(file_name)));
    }
    candidates.into_iter().find(|path| path.is_file())
}
