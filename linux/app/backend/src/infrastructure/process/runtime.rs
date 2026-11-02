//! 外部工具的串行执行、日志、取消与设备轮询。

use super::command::{configure_tool_process, format_command, terminate_tool_process};
#[cfg(test)]
use super::logging::resolve_tool_log_directory;
use super::logging::{
    append_log_lines, create_log_file, initial_tool_log_directory, next_operation_log_path,
    prepare_log_redirect, purge_stale_runtime_directories, remove_runtime_directory,
    runtime_tool_home_directory,
};
use super::monitor::spawn_device_poller;
use super::terminal::sanitize_terminal_text;
use crate::application::{
    DeviceMonitor, OperationExecutor, OperationLog, ToolLocator, ToolProtocol,
};
use crate::domain::{DeviceSnapshot, ExecutionEvent, ExecutionPlan};
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, TryLockError};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) struct ProcessRuntime {
    tool_path: Arc<Mutex<Option<PathBuf>>>,
    tool_log_directory: Mutex<PathBuf>,
    tool_home_directory: PathBuf,
    tool_log_relative_path: PathBuf,
    operation_log_state: Arc<Mutex<OperationLogState>>,
    event_tx: mpsc::Sender<ExecutionEvent>,
    event_rx: mpsc::Receiver<ExecutionEvent>,
    device_rx: mpsc::Receiver<DeviceSnapshot>,
    active: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
    command_gate: Arc<Mutex<()>>,
    stop_poller: Arc<AtomicBool>,
    protocol: Arc<dyn ToolProtocol>,
}

/// 一次操作期间保持打开的日志文件；避免每行日志重新 open/close。
struct OperationLogFile {
    path: PathBuf,
    file: std::fs::File,
}

#[derive(Default)]
struct OperationLogState {
    active: Option<OperationLogFile>,
    last: Option<PathBuf>,
}

fn finish_operation_log_state(state: &Mutex<OperationLogState>) {
    let mut state = lock_or_recover(state);
    state.last = state.active.take().map(|log| log.path);
}

/// 锁中毒时恢复数据而不是放弃互斥保护：锁持有者 panic 不应让整个运行时失能。
fn lock_or_recover<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct RunContext {
    tx: mpsc::Sender<ExecutionEvent>,
    active: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    child_slot: Arc<Mutex<Option<Child>>>,
    command_gate: Arc<Mutex<()>>,
    protocol: Arc<dyn ToolProtocol>,
    tool_home_directory: PathBuf,
    operation_log_state: Arc<Mutex<OperationLogState>>,
}

impl ProcessRuntime {
    pub(crate) fn new(
        tool_path: Option<PathBuf>,
        protocol: Arc<dyn ToolProtocol>,
        tool_log_relative_path: PathBuf,
    ) -> Self {
        let tool_path = Arc::new(Mutex::new(tool_path));
        let tool_home_directory = runtime_tool_home_directory();
        purge_stale_runtime_directories(&tool_home_directory);
        let tool_log_directory = Mutex::new(initial_tool_log_directory(
            &tool_home_directory,
            &tool_log_relative_path,
        ));
        let active = Arc::new(AtomicBool::new(false));
        let stop_poller = Arc::new(AtomicBool::new(false));
        let command_gate = Arc::new(Mutex::new(()));
        let (event_tx, event_rx) = mpsc::channel();
        let (device_tx, device_rx) = mpsc::channel();

        spawn_device_poller(
            Arc::clone(&tool_path),
            tool_home_directory.clone(),
            Arc::clone(&active),
            Arc::clone(&command_gate),
            Arc::clone(&stop_poller),
            Arc::clone(&protocol),
            device_tx,
        );

        Self {
            tool_path,
            tool_log_directory,
            tool_home_directory,
            tool_log_relative_path,
            operation_log_state: Arc::new(Mutex::new(OperationLogState::default())),
            event_tx,
            event_rx,
            device_rx,
            active,
            cancelled: Arc::new(AtomicBool::new(false)),
            child: Arc::new(Mutex::new(None)),
            command_gate,
            stop_poller,
            protocol,
        }
    }
}

impl ToolLocator for ProcessRuntime {
    fn tool_path(&self) -> Option<PathBuf> {
        lock_or_recover(&self.tool_path).clone()
    }

    fn set_tool_path(&self, path: Option<PathBuf>) {
        *lock_or_recover(&self.tool_path) = path;
    }
}

impl OperationLog for ProcessRuntime {
    fn tool_log_directory(&self) -> PathBuf {
        lock_or_recover(&self.tool_log_directory).clone()
    }

    fn set_tool_log_directory(&self, path: PathBuf) -> Result<(), String> {
        if path.as_os_str().is_empty() {
            return Err("工具日志目录不能为空".to_owned());
        }
        if self.active.load(Ordering::Acquire) {
            return Err("当前任务运行期间不能切换工具日志目录".to_owned());
        }
        // 不与任务或设备扫描长时间串行等待：扫描短暂持锁时最多重试约 1 秒，
        // 避免扫描或任务挂起时冻结 UI 线程。
        let deadline = Instant::now() + Duration::from_millis(1_000);
        let command_guard = loop {
            match self.command_gate.try_lock() {
                Ok(guard) => break guard,
                Err(TryLockError::Poisoned(poisoned)) => break poisoned.into_inner(),
                Err(TryLockError::WouldBlock) => {
                    if self.active.load(Ordering::Acquire) {
                        return Err("当前任务运行期间不能切换工具日志目录".to_owned());
                    }
                    if Instant::now() >= deadline {
                        return Err("设备扫描正在进行，请稍后重试".to_owned());
                    }
                    thread::sleep(Duration::from_millis(25));
                }
            }
        };
        let _command_guard = command_guard;
        if self.active.load(Ordering::Acquire) {
            return Err("当前任务运行期间不能切换工具日志目录".to_owned());
        }
        let path = prepare_log_redirect(
            &self.tool_home_directory,
            &self.tool_log_relative_path,
            &path,
        )?;
        *lock_or_recover(&self.tool_log_directory) = path;
        Ok(())
    }

    fn begin_operation_log(&self, operation: &str) -> Result<PathBuf, String> {
        let path = next_operation_log_path(&self.tool_log_directory())?;
        let mut file = create_log_file(&path)?;
        append_log_lines(&mut file, "INFO", &format!("开始操作：{operation}"))?;
        let mut state = lock_or_recover(&self.operation_log_state);
        state.active = Some(OperationLogFile {
            path: path.clone(),
            file,
        });
        Ok(path)
    }

    fn finish_operation_log(&self) {
        finish_operation_log_state(&self.operation_log_state);
    }

    fn operation_log_path(&self) -> Option<PathBuf> {
        let state = lock_or_recover(&self.operation_log_state);
        state
            .active
            .as_ref()
            .map(|log| log.path.clone())
            .or_else(|| state.last.clone())
    }

    fn append_operation_log(&self, level: &str, text: &str) -> Result<(), String> {
        let mut state = lock_or_recover(&self.operation_log_state);
        match state.active.as_mut() {
            Some(log) => append_log_lines(&mut log.file, level, text),
            None => Ok(()),
        }
    }
}

impl OperationExecutor for ProcessRuntime {
    fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    fn start(&self, plan: ExecutionPlan) -> Result<(), String> {
        if plan.commands.is_empty() {
            return Err("任务中没有可执行命令".to_owned());
        }
        if self
            .active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("已有任务正在执行".to_owned());
        }

        let Some(tool_path) = self.tool_path() else {
            self.active.store(false, Ordering::Release);
            return Err("未配置外部工具".to_owned());
        };

        self.cancelled.store(false, Ordering::Release);
        let context = RunContext {
            tx: self.event_tx.clone(),
            active: Arc::clone(&self.active),
            cancelled: Arc::clone(&self.cancelled),
            child_slot: Arc::clone(&self.child),
            command_gate: Arc::clone(&self.command_gate),
            protocol: Arc::clone(&self.protocol),
            tool_home_directory: self.tool_home_directory.clone(),
            operation_log_state: Arc::clone(&self.operation_log_state),
        };

        thread::spawn(move || {
            run_plan(tool_path, plan, context);
        });
        Ok(())
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        let mut slot = lock_or_recover(&self.child);
        if let Some(child) = slot.as_mut() {
            terminate_tool_process(child);
        }
    }

    fn drain_events(&self) -> Vec<ExecutionEvent> {
        self.event_rx.try_iter().collect()
    }
}

impl DeviceMonitor for ProcessRuntime {
    fn latest_device_snapshot(&self) -> Option<DeviceSnapshot> {
        self.device_rx.try_iter().last()
    }
}

impl Drop for ProcessRuntime {
    fn drop(&mut self) {
        self.stop_poller.store(true, Ordering::Release);
        self.cancel();
        remove_runtime_directory(&self.tool_home_directory);
    }
}

fn run_plan(tool_path: PathBuf, plan: ExecutionPlan, context: RunContext) {
    let RunContext {
        tx,
        active,
        cancelled,
        child_slot,
        command_gate,
        protocol,
        tool_home_directory,
        operation_log_state,
    } = context;
    let Ok(_command_guard) = command_gate.lock() else {
        active.store(false, Ordering::Release);
        let _ = tx.send(ExecutionEvent::TaskFinished {
            name: plan.name,
            success: false,
            cancelled: false,
            message: "无法获取工具执行锁".to_owned(),
        });
        return;
    };
    let total = plan.commands.len();
    let task_name = plan.name.clone();
    let _ = tx.send(ExecutionEvent::TaskStarted {
        name: task_name.clone(),
        steps: total,
    });

    let mut success = true;
    let mut cancelled_by_user = false;
    let mut final_message = "操作完成".to_owned();

    for (index, command) in plan.commands.into_iter().enumerate() {
        if cancelled.load(Ordering::Acquire) {
            success = false;
            cancelled_by_user = true;
            final_message = "操作已由用户中止".to_owned();
            break;
        }

        let display = format_command(&tool_path, &command.args);
        let _ = tx.send(ExecutionEvent::StepStarted {
            index: index + 1,
            total,
            label: command.label.clone(),
            command: display,
        });

        let mut process = Command::new(&tool_path);
        process
            .args(&command.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_tool_process(&mut process, &tool_path, &tool_home_directory);
        let mut child = match process.spawn() {
            Ok(child) => child,
            Err(error) => {
                success = false;
                final_message = format!("无法启动外部工具：{error}");
                break;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        *lock_or_recover(&child_slot) = Some(child);

        let stdout_reader = stdout.map(|stream| {
            spawn_output_reader(
                stream,
                false,
                index,
                total,
                tx.clone(),
                Arc::clone(&protocol),
            )
        });
        let stderr_reader = stderr.map(|stream| {
            spawn_output_reader(
                stream,
                true,
                index,
                total,
                tx.clone(),
                Arc::clone(&protocol),
            )
        });

        let exit_status = loop {
            if cancelled.load(Ordering::Acquire) {
                if let Some(child) = lock_or_recover(&child_slot).as_mut() {
                    terminate_tool_process(child);
                }
            }

            let status = lock_or_recover(&child_slot)
                .as_mut()
                .ok_or("子进程状态丢失")
                .and_then(|child| child.try_wait().map_err(|_| "无法读取子进程状态"));
            match status {
                Ok(Some(status)) => break Some(status),
                Ok(None) => thread::sleep(Duration::from_millis(60)),
                Err(error) => {
                    success = false;
                    final_message = error.to_owned();
                    break None;
                }
            }
        };

        let stdout_failure = join_output_reader(stdout_reader);
        let stderr_failure = join_output_reader(stderr_reader);
        *lock_or_recover(&child_slot) = None;

        if cancelled.load(Ordering::Acquire) {
            success = false;
            cancelled_by_user = true;
            final_message = "操作已由用户中止".to_owned();
            break;
        }
        if let Some(failure) = stdout_failure.or(stderr_failure) {
            success = false;
            final_message = format!("步骤“{}”失败：{failure}", command.label);
            break;
        }
        if !exit_status.is_some_and(|status| status.success()) {
            if success {
                success = false;
                final_message = format!("步骤“{}”执行失败", command.label);
            }
            break;
        }
        let _ = tx.send(ExecutionEvent::Progress((index + 1) as f32 / total as f32));
    }

    // 在发送 TaskFinished 之前由后端关闭日志：前端可能在收到事件后立即开始
    // 下一个任务，届时不应误关新任务的日志。
    finish_operation_log_state(&operation_log_state);
    active.store(false, Ordering::Release);
    let _ = tx.send(ExecutionEvent::TaskFinished {
        name: task_name,
        success,
        cancelled: cancelled_by_user,
        message: final_message,
    });
}

fn join_output_reader(reader: Option<thread::JoinHandle<Option<String>>>) -> Option<String> {
    match reader?.join() {
        Ok(failure) => failure,
        Err(_) => Some("工具输出读取线程异常终止".to_owned()),
    }
}

fn spawn_output_reader<R: Read + Send + 'static>(
    stream: R,
    stderr: bool,
    step_index: usize,
    step_count: usize,
    tx: mpsc::Sender<ExecutionEvent>,
    protocol: Arc<dyn ToolProtocol>,
) -> thread::JoinHandle<Option<String>> {
    thread::spawn(move || {
        let mut failure = None;
        for line in BufReader::new(stream).lines() {
            let line = match line {
                Ok(line) => line,
                Err(error) => {
                    if failure.is_none() {
                        failure = Some(format!("读取工具输出失败：{error}"));
                    }
                    break;
                }
            };
            let line = sanitize_terminal_text(&line);
            if line.is_empty() {
                continue;
            }
            if let Some(progress) = protocol.parse_progress(&line) {
                let overall = (step_index as f32 + progress) / step_count as f32;
                let _ = tx.send(ExecutionEvent::Progress(overall));
            }
            if failure.is_none() {
                failure = protocol.parse_failure(&line);
            }
            let _ = tx.send(ExecutionEvent::Output { text: line, stderr });
        }
        failure
    })
}

#[cfg(test)]
mod tests {
    use super::{
        configure_tool_process, prepare_log_redirect, resolve_tool_log_directory, run_plan,
        runtime_tool_home_directory, sanitize_terminal_text, ProcessRuntime, RunContext,
    };
    use crate::application::{OperationLog, ToolProtocol};
    use crate::domain::{CommandSpec, DeviceSnapshot, ExecutionEvent, ExecutionPlan};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc, Mutex};

    struct TestProtocol;

    #[test]
    fn configures_external_tool_home_without_changing_its_working_directory_contract() {
        let mut command = std::process::Command::new("/opt/rkflash/tool");
        configure_tool_process(
            &mut command,
            std::path::Path::new("/opt/rkflash/tool"),
            std::path::Path::new("/tmp/rkflash-dev"),
        );

        assert_eq!(
            command.get_current_dir(),
            Some(std::path::Path::new("/opt/rkflash"))
        );
        let home = command
            .get_envs()
            .find(|(name, _)| *name == "HOME")
            .and_then(|(_, value)| value);
        assert_eq!(home, Some(std::ffi::OsStr::new("/tmp/rkflash-dev")));
    }

    #[test]
    fn strips_terminal_cursor_and_style_sequences_from_gui_output() {
        assert_eq!(
            sanitize_terminal_text("\x1b[1A\x1b[2KDownload Image... (55%)"),
            "Download Image... (55%)"
        );
        assert_eq!(
            sanitize_terminal_text("\x1b[32mDownload Image... (56%)\x1b[0m"),
            "Download Image... (56%)"
        );
        assert_eq!(
            sanitize_terminal_text("\x1b]0;vendor-tool\x07Done\r"),
            "Done"
        );
    }

    impl ToolProtocol for TestProtocol {
        fn device_scan_command(&self) -> CommandSpec {
            CommandSpec::new("scan", ["-c", "true"])
        }

        fn parse_device_list(&self, _text: &str) -> DeviceSnapshot {
            DeviceSnapshot::default()
        }

        fn parse_progress(&self, _line: &str) -> Option<f32> {
            None
        }

        fn parse_failure(&self, line: &str) -> Option<String> {
            line.contains("Fail").then(|| line.to_owned())
        }
    }

    #[test]
    fn textual_failure_overrides_zero_exit_status() {
        let (tx, rx) = mpsc::channel();
        let active = Arc::new(AtomicBool::new(true));
        let context = RunContext {
            tx,
            active: Arc::clone(&active),
            cancelled: Arc::new(AtomicBool::new(false)),
            child_slot: Arc::new(Mutex::new(None)),
            command_gate: Arc::new(Mutex::new(())),
            protocol: Arc::new(TestProtocol),
            tool_home_directory: PathBuf::from("/tmp/rkflash-test-home"),
            operation_log_state: Arc::new(Mutex::new(Default::default())),
        };
        let plan = ExecutionPlan::single(
            "读取 Flash ID",
            CommandSpec::new(
                "读取 Flash ID",
                ["-c", "printf 'Read flash ID Fail!\\n'; exit 0"],
            ),
        );

        run_plan(PathBuf::from("/bin/sh"), plan, context);

        let finished = rx.try_iter().find_map(|event| match event {
            ExecutionEvent::TaskFinished {
                success,
                message,
                cancelled,
                ..
            } => Some((success, cancelled, message)),
            _ => None,
        });
        assert_eq!(
            finished,
            Some((
                false,
                false,
                "步骤“读取 Flash ID”失败：Read flash ID Fail!".to_owned()
            ))
        );
        assert!(!active.load(Ordering::Acquire));
    }

    #[test]
    fn resolves_explicit_or_temporary_tool_log_directory() {
        assert_eq!(
            resolve_tool_log_directory(Some("/tmp/rkflash-dev".into()), PathBuf::from("/tmp")),
            PathBuf::from("/tmp/rkflash-dev")
        );
        assert_eq!(
            resolve_tool_log_directory(None, PathBuf::from("/tmp")),
            PathBuf::from("/tmp/rkflash/log")
        );
    }

    #[test]
    fn maps_vendor_tool_fixed_log_path_to_selected_directory() {
        let tool_home = runtime_tool_home_directory();
        let selected_log_directory = tool_home.with_extension("selected-log");
        let active_log_directory = prepare_log_redirect(
            &tool_home,
            std::path::Path::new("vendor-tool/log"),
            &selected_log_directory,
        )
        .expect("log redirect");

        assert_eq!(
            std::fs::read_link(tool_home.join("vendor-tool/log")).expect("log symlink"),
            active_log_directory
        );

        std::fs::remove_dir_all(&tool_home).expect("remove test tool home");
        std::fs::remove_dir_all(&selected_log_directory).expect("remove test log directory");
    }

    #[test]
    fn creates_one_timestamped_log_for_each_operation() {
        let directory = runtime_tool_home_directory().with_extension("operation-log");
        let runtime = ProcessRuntime::new(
            None,
            Arc::new(TestProtocol),
            PathBuf::from("vendor-tool/log"),
        );
        runtime
            .set_tool_log_directory(directory.clone())
            .expect("set operation log directory");
        let path = runtime
            .begin_operation_log("下载镜像")
            .expect("begin operation log");
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("UTF-8 log name");
        assert!(file_name.ends_with(".log"));
        assert_eq!(file_name.len(), 23);
        assert_eq!(&file_name[8..9], "-");
        assert_eq!(&file_name[15..16], "-");

        runtime
            .append_operation_log("INFO", "开始烧录")
            .expect("first log entry");
        runtime
            .append_operation_log("ERROR", "写入失败\n设备已断开")
            .expect("multiline log entry");
        runtime.finish_operation_log();
        assert_eq!(runtime.operation_log_path(), Some(path.clone()));

        let content = std::fs::read_to_string(&path).expect("read operation log");
        assert!(content.contains("[INFO] 开始操作：下载镜像"));
        assert!(content.contains("[INFO] 开始烧录"));
        assert!(content.contains("[ERROR] 写入失败"));
        assert!(content.contains("[ERROR] 设备已断开"));

        let tool_home = runtime.tool_home_directory.clone();
        drop(runtime);
        // Drop 会清理本次运行的 HOME 隔离目录；重复删除不视为错误。
        let _ = std::fs::remove_dir_all(&tool_home);
        let _ = std::fs::remove_dir_all(&directory);
    }
}
