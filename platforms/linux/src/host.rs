//! Unix 的进程与日志实现：独立进程组、localtime_r，以及 HOME 下的日志软链接映射。

use rkflash_runtime::{LocalTime, OsHost, ToolProcessContext};
use std::fs;
use std::os::unix::fs::symlink;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct UnixHost;

impl OsHost for UnixHost {
    fn configure_tool_process(&self, command: &mut Command, context: &ToolProcessContext<'_>) {
        // upgrade_tool 在自身所在目录查找配置文件，因此保持在工具目录中运行。
        if let Some(directory) = context.tool_path.parent() {
            command.current_dir(directory);
        }
        // 让工具进程成为独立进程组的组长：终止时可以按组清理它派生的孙进程，
        // 避免残留进程继续占用 USB 设备或持有输出管道。
        command.process_group(0);
    }

    fn terminate_tool_process(&self, child: &mut Child) {
        let process_group_id = child.id() as libc::c_int;
        // SAFETY: `killpg` 只对已验证的整数进程组号发送信号。
        if unsafe { libc::killpg(process_group_id, libc::SIGKILL) } != 0 {
            let _ = child.kill();
        }
    }

    fn process_has_exited(&self, pid: &str) -> bool {
        let Ok(pid) = pid.parse::<libc::pid_t>() else {
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

    fn local_time(&self) -> Option<LocalTime> {
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
        Some(LocalTime {
            year: local.tm_year + 1900,
            month: local.tm_mon + 1,
            day: local.tm_mday,
            hour: local.tm_hour,
            minute: local.tm_min,
            second: local.tm_sec,
            millis: duration.subsec_millis(),
        })
    }

    /// upgrade_tool 固定把日志写到 `$HOME/<相对路径>`；在隔离 HOME 中放一个指向用户日志目录的软链接。
    fn prepare_log_redirect(
        &self,
        tool_home_directory: &Path,
        tool_log_relative_path: &Path,
        log_directory: &Path,
    ) -> Result<PathBuf, String> {
        fs::create_dir_all(log_directory).map_err(|error| {
            format!("无法创建工具日志目录 {}：{error}", log_directory.display())
        })?;
        let log_directory = log_directory.canonicalize().map_err(|error| {
            format!("无法解析工具日志目录 {}：{error}", log_directory.display())
        })?;

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
            fs::remove_file(&pending_link).map_err(|error| {
                format!("无法更新工具日志映射 {}：{error}", pending_link.display())
            })?;
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
}

#[cfg(test)]
mod tests {
    use super::UnixHost;
    use rkflash_runtime::{OsHost, ToolProcessContext};
    use std::path::Path;

    #[test]
    fn runs_tool_from_its_own_directory() {
        let mut command = std::process::Command::new("/opt/rkflash/tool");
        UnixHost.configure_tool_process(
            &mut command,
            &ToolProcessContext {
                tool_path: Path::new("/opt/rkflash/tool"),
                tool_home_directory: Path::new("/tmp/rkflash-dev"),
                tool_log_directory: None,
            },
        );
        assert_eq!(command.get_current_dir(), Some(Path::new("/opt/rkflash")));
    }

    #[test]
    fn maps_vendor_tool_fixed_log_path_to_selected_directory() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tool_home = std::env::temp_dir().join(format!("rkflash-unix-host-{nonce}"));
        let selected = tool_home.with_extension("selected-log");
        let active = UnixHost
            .prepare_log_redirect(&tool_home, Path::new("vendor-tool/log"), &selected)
            .expect("log redirect");
        assert_eq!(
            std::fs::read_link(tool_home.join("vendor-tool/log")).expect("log symlink"),
            active
        );
        std::fs::remove_dir_all(&tool_home).expect("remove test tool home");
        std::fs::remove_dir_all(&selected).expect("remove test log directory");
    }

    #[test]
    fn reads_local_time_and_detects_missing_process() {
        assert!(UnixHost.local_time().is_some());
        assert!(!UnixHost.process_has_exited(&std::process::id().to_string()));
    }
}
