//! 只读设备轮询适配器；扫描不会隐式切换设备模式。

use super::command::configure_tool_process;
use crate::host::{OsHost, ToolProcessContext};
use rkflash_application::domain::DeviceSnapshot;
use rkflash_application::ToolProtocol;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// 单次设备扫描的最长等待。工具在 USB 设备异常时可能永久阻塞，
/// 超时后按进程组终止，保证轮询线程和后续任务不会被卡死。
const SCAN_TIMEOUT: Duration = Duration::from_secs(10);

/// 设备轮询线程与运行时共享的状态。
pub(super) struct DevicePoller {
    pub tool_path: Arc<Mutex<Option<PathBuf>>>,
    pub tool_home_directory: PathBuf,
    /// 有任务执行时暂停轮询，避免与任务争用 USB 设备。
    pub active: Arc<AtomicBool>,
    pub command_gate: Arc<Mutex<()>>,
    pub stop: Arc<AtomicBool>,
    pub protocol: Arc<dyn ToolProtocol>,
    pub host: Arc<dyn OsHost>,
}

pub(super) fn spawn_device_poller(poller: DevicePoller, tx: mpsc::Sender<DeviceSnapshot>) {
    let DevicePoller {
        tool_path,
        tool_home_directory,
        active,
        command_gate,
        stop,
        protocol,
        host,
    } = poller;
    thread::spawn(move || {
        while !stop.load(Ordering::Acquire) {
            if !active.load(Ordering::Acquire) {
                let path = tool_path
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clone();
                let snapshot = match path {
                    Some(path) => scan_devices(
                        host.as_ref(),
                        &path,
                        &tool_home_directory,
                        protocol.as_ref(),
                        &command_gate,
                    ),
                    None => DeviceSnapshot {
                        devices: Vec::new(),
                        error: Some("未配置外部工具".to_owned()),
                    },
                };
                if tx.send(snapshot).is_err() {
                    return;
                }
            }

            for _ in 0..20 {
                if stop.load(Ordering::Acquire) {
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    });
}

fn scan_devices(
    host: &dyn OsHost,
    tool_path: &Path,
    tool_home_directory: &Path,
    protocol: &dyn ToolProtocol,
    command_gate: &Mutex<()>,
) -> DeviceSnapshot {
    let Ok(_command_guard) = command_gate.lock() else {
        return DeviceSnapshot {
            devices: Vec::new(),
            error: Some("无法获取设备扫描锁".to_owned()),
        };
    };
    let scan = protocol.device_scan_command();
    let mut command = Command::new(tool_path);
    command
        .args(scan.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_tool_process(
        host,
        &mut command,
        &ToolProcessContext {
            tool_path,
            tool_home_directory,
            tool_log_directory: None,
        },
    );
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return DeviceSnapshot {
                devices: Vec::new(),
                error: Some(format!("设备扫描失败：{error}")),
            }
        }
    };

    let deadline = Instant::now() + SCAN_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    host.terminate_tool_process(&mut child);
                    let _ = child.wait();
                    return DeviceSnapshot {
                        devices: Vec::new(),
                        error: Some(
                            "设备扫描超时，已终止本次扫描；请检查设备连接或更换工具版本".to_owned(),
                        ),
                    };
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                return DeviceSnapshot {
                    devices: Vec::new(),
                    error: Some(format!("设备扫描失败：{error}")),
                }
            }
        }
    }

    match child.wait_with_output() {
        Ok(output) => {
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            if !output.stderr.is_empty() {
                text.push_str(&String::from_utf8_lossy(&output.stderr));
            }
            let mut snapshot = protocol.parse_device_list(&text);
            if !output.status.success() && snapshot.devices.is_empty() {
                snapshot.error = Some(text.trim().to_owned());
            }
            snapshot
        }
        Err(error) => DeviceSnapshot {
            devices: Vec::new(),
            error: Some(format!("读取设备扫描输出失败：{error}")),
        },
    }
}
