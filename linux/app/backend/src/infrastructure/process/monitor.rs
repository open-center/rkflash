//! 只读设备轮询适配器；扫描不会隐式切换设备模式。

use super::command::{configure_tool_process, terminate_tool_process};
use crate::application::ToolProtocol;
use crate::domain::DeviceSnapshot;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// 单次设备扫描的最长等待。工具在 USB 设备异常时可能永久阻塞，
/// 超时后按进程组终止，保证轮询线程和后续任务不会被卡死。
const SCAN_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) fn spawn_device_poller(
    tool_path: Arc<Mutex<Option<PathBuf>>>,
    tool_home_directory: PathBuf,
    active: Arc<AtomicBool>,
    command_gate: Arc<Mutex<()>>,
    stop: Arc<AtomicBool>,
    protocol: Arc<dyn ToolProtocol>,
    tx: mpsc::Sender<DeviceSnapshot>,
) {
    thread::spawn(move || {
        while !stop.load(Ordering::Acquire) {
            if !active.load(Ordering::Acquire) {
                let path = tool_path
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clone();
                let snapshot = match path {
                    Some(path) => scan_devices(
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
    configure_tool_process(&mut command, tool_path, tool_home_directory);
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
                    terminate_tool_process(&mut child);
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
