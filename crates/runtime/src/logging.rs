//! 外部工具 HOME 隔离、日志目录映射与操作日志文件；平台差异经 [`OsHost`] 注入。

use crate::host::OsHost;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn runtime_tool_home_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir()
        .join("rkflash/.runtime")
        .join(format!("{}-{nonce}", std::process::id()))
}

/// 清理历史运行留下的工具 HOME 隔离目录。目录名形如 `<pid>-<nonce>`；
/// 只删除对应进程已经退出的目录。
pub(super) fn purge_stale_runtime_directories(host: &dyn OsHost, current: &Path) {
    let Some(parent) = current.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some((pid_text, _)) = name.split_once('-') else {
            continue;
        };
        if host.process_has_exited(pid_text) {
            let _ = fs::remove_dir_all(&path);
        }
    }
}

pub(super) fn remove_runtime_directory(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

pub(super) fn initial_tool_log_directory(
    host: &dyn OsHost,
    tool_home_directory: &Path,
    tool_log_relative_path: &Path,
) -> PathBuf {
    let preferred = default_tool_log_directory();
    if let Ok(path) =
        host.prepare_log_redirect(tool_home_directory, tool_log_relative_path, &preferred)
    {
        return path;
    }

    let fallback = std::env::temp_dir().join("rkflash/log");
    if fallback == preferred {
        return fallback;
    }
    host.prepare_log_redirect(tool_home_directory, tool_log_relative_path, &fallback)
        .unwrap_or(fallback)
}

pub(super) fn default_tool_log_directory() -> PathBuf {
    resolve_tool_log_directory(std::env::var_os("RKFLASH_LOG_DIR"), std::env::temp_dir())
}

pub(super) fn resolve_tool_log_directory(
    configured: Option<OsString>,
    temp_directory: PathBuf,
) -> PathBuf {
    configured
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| temp_directory.join("rkflash/log"))
}

pub(super) fn next_operation_log_path(
    host: &dyn OsHost,
    directory: &Path,
) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("无法创建操作日志目录 {}：{error}", directory.display()))?;
    let timestamp = file_timestamp(host);
    let direct = directory.join(format!("{timestamp}.log"));
    if !direct.exists() {
        return Ok(direct);
    }
    for index in 1..=99 {
        let candidate = directory.join(format!("{timestamp}-{index:02}.log"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "同一时间戳下的操作日志过多：{}",
        directory.display()
    ))
}

pub(super) fn create_log_file(path: &Path) -> Result<fs::File, String> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("无法打开操作日志 {}：{error}", path.display()))
}

/// 向已打开的日志文件追加带时间戳的条目；由调用方负责持有文件句柄。
pub(super) fn append_log_lines(
    host: &dyn OsHost,
    file: &mut fs::File,
    level: &str,
    text: &str,
) -> Result<(), String> {
    let timestamp = display_timestamp(host);
    if text.is_empty() {
        writeln!(file, "[{timestamp}] [{level}]")
            .map_err(|error| format!("无法写入操作日志：{error}"))?;
    } else {
        for line in text.lines() {
            writeln!(file, "[{timestamp}] [{level}] {line}")
                .map_err(|error| format!("无法写入操作日志：{error}"))?;
        }
    }
    file.flush()
        .map_err(|error| format!("无法刷新操作日志：{error}"))
}

fn file_timestamp(host: &dyn OsHost) -> String {
    host.local_time()
        .map(|t| {
            format!(
                "{:04}{:02}{:02}-{:02}{:02}{:02}-{:03}",
                t.year, t.month, t.day, t.hour, t.minute, t.second, t.millis
            )
        })
        .unwrap_or_else(|| {
            format!(
                "unix-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            )
        })
}

fn display_timestamp(host: &dyn OsHost) -> String {
    host.local_time()
        .map(|t| {
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
                t.year, t.month, t.day, t.hour, t.minute, t.second, t.millis
            )
        })
        .unwrap_or_else(|| {
            format!(
                "unix:{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            )
        })
}
