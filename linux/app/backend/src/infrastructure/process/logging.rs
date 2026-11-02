//! 外部工具 HOME 隔离、日志目录映射与操作日志文件。

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::symlink;

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
/// 进程已不存在（ESRCH）的目录才可以删除。
pub(super) fn purge_stale_runtime_directories(current: &Path) {
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
        if process_has_exited(pid_text) {
            let _ = fs::remove_dir_all(&path);
        }
    }
}

#[cfg(unix)]
fn process_has_exited(pid_text: &str) -> bool {
    let Ok(pid) = pid_text.parse::<libc::pid_t>() else {
        return false;
    };
    if pid <= 0 {
        return false;
    }
    // SAFETY: 信号 0 只探测进程是否存在，不产生任何副作用。
    if unsafe { libc::kill(pid, 0) } == 0 {
        return false;
    }
    // EPERM 等情况说明进程仍存在，只是属于其他用户。
    std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

#[cfg(windows)]
fn process_has_exited(pid_text: &str) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_INVALID_PARAMETER};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    const STILL_ACTIVE: u32 = 259;

    let Ok(pid) = pid_text.parse::<u32>() else {
        return false;
    };
    if pid == 0 {
        return false;
    }
    // SAFETY: 只以查询权限打开进程句柄，读取退出码后立即关闭。
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            // 进程号不存在时返回 ERROR_INVALID_PARAMETER；拒绝访问说明进程仍存在。
            return GetLastError() == ERROR_INVALID_PARAMETER;
        }
        let mut exit_code = 0u32;
        let queried = GetExitCodeProcess(handle, &mut exit_code) != 0;
        CloseHandle(handle);
        queried && exit_code != STILL_ACTIVE
    }
}

pub(super) fn remove_runtime_directory(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

pub(super) fn initial_tool_log_directory(
    tool_home_directory: &Path,
    tool_log_relative_path: &Path,
) -> PathBuf {
    let preferred = default_tool_log_directory();
    if let Ok(path) = prepare_log_redirect(tool_home_directory, tool_log_relative_path, &preferred)
    {
        return path;
    }

    let fallback = std::env::temp_dir().join("rkflash/log");
    if fallback == preferred {
        return fallback;
    }
    prepare_log_redirect(tool_home_directory, tool_log_relative_path, &fallback).unwrap_or(fallback)
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

pub(super) fn next_operation_log_path(directory: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("无法创建操作日志目录 {}：{error}", directory.display()))?;
    let timestamp = file_timestamp();
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
pub(super) fn append_log_lines(file: &mut fs::File, level: &str, text: &str) -> Result<(), String> {
    let timestamp = display_timestamp();
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

fn file_timestamp() -> String {
    local_time_parts()
        .map(|(year, month, day, hour, minute, second, millis)| {
            format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}-{millis:03}")
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

fn display_timestamp() -> String {
    local_time_parts()
        .map(|(year, month, day, hour, minute, second, millis)| {
            format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millis:03}")
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

#[cfg(unix)]
fn local_time_parts() -> Option<(i32, i32, i32, i32, i32, i32, u32)> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let seconds = libc::time_t::try_from(duration.as_secs()).ok()?;
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: both pointers remain valid for the call, and a non-null return
    // guarantees that `local` has been fully initialized.
    let result = unsafe { libc::localtime_r(&seconds, local.as_mut_ptr()) };
    if result.is_null() {
        return None;
    }
    // SAFETY: checked the non-null return from `localtime_r` above.
    let local = unsafe { local.assume_init() };
    Some((
        local.tm_year + 1900,
        local.tm_mon + 1,
        local.tm_mday,
        local.tm_hour,
        local.tm_min,
        local.tm_sec,
        duration.subsec_millis(),
    ))
}

#[cfg(windows)]
fn local_time_parts() -> Option<(i32, i32, i32, i32, i32, i32, u32)> {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    // SAFETY: GetLocalTime 只写入调用方提供的 SYSTEMTIME 结构。
    let local = unsafe {
        let mut local = std::mem::zeroed();
        GetLocalTime(&mut local);
        local
    };
    Some((
        i32::from(local.wYear),
        i32::from(local.wMonth),
        i32::from(local.wDay),
        i32::from(local.wHour),
        i32::from(local.wMinute),
        i32::from(local.wSecond),
        u32::from(local.wMilliseconds),
    ))
}

/// Windows 创建软链接需要管理员或开发者模式，因此不做 HOME 映射：
/// 日志目录在启动工具时通过环境变量直接传给 rkdeveloptool。
#[cfg(windows)]
pub(super) fn prepare_log_redirect(
    _tool_home_directory: &Path,
    _tool_log_relative_path: &Path,
    log_directory: &Path,
) -> Result<PathBuf, String> {
    fs::create_dir_all(log_directory)
        .map_err(|error| format!("无法创建工具日志目录 {}：{error}", log_directory.display()))?;
    std::path::absolute(log_directory)
        .map_err(|error| format!("无法解析工具日志目录 {}：{error}", log_directory.display()))
}

#[cfg(unix)]
pub(super) fn prepare_log_redirect(
    tool_home_directory: &Path,
    tool_log_relative_path: &Path,
    log_directory: &Path,
) -> Result<PathBuf, String> {
    fs::create_dir_all(log_directory)
        .map_err(|error| format!("无法创建工具日志目录 {}：{error}", log_directory.display()))?;
    let log_directory = log_directory
        .canonicalize()
        .map_err(|error| format!("无法解析工具日志目录 {}：{error}", log_directory.display()))?;

    let link = tool_home_directory.join(tool_log_relative_path);
    let Some(tool_state_directory) = link.parent() else {
        return Err("工具日志相对路径缺少父目录".to_owned());
    };
    fs::create_dir_all(tool_state_directory).map_err(|error| {
        format!(
            "无法创建工具兼容目录 {}：{error}",
            tool_state_directory.display()
        )
    })?;

    let pending_link = link.with_extension("next");
    if pending_link.symlink_metadata().is_ok() {
        fs::remove_file(&pending_link)
            .map_err(|error| format!("无法更新工具日志映射 {}：{error}", pending_link.display()))?;
    }
    symlink(&log_directory, &pending_link).map_err(|error| {
        format!(
            "无法创建工具日志映射 {} -> {}：{error}",
            pending_link.display(),
            log_directory.display()
        )
    })?;
    fs::rename(&pending_link, &link).map_err(|error| {
        let _ = fs::remove_file(&pending_link);
        format!(
            "无法启用工具日志映射 {} -> {}：{error}",
            link.display(),
            log_directory.display()
        )
    })?;
    Ok(log_directory)
}
