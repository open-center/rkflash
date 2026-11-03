//! 任务进度与取消 ↔ `TaskApi`。

use super::super::*;

impl RkFlashApp {
    pub(in crate::desktop) fn is_task_running(&self) -> bool {
        self.application.is_task_running()
    }

    pub(in crate::desktop) fn cancel_task(&mut self) {
        self.application.cancel_task();
        self.push_log(LogLevel::Warning, "正在请求停止当前任务……");
    }

    pub(in crate::desktop) fn process_task_events(&mut self) {
        for event in self.application.drain_task_events() {
            match event {
                ExecutionEvent::TaskStarted { name, steps } => {
                    self.progress = 0.0;
                    self.status_text = format!("正在执行：{name}");
                    if let Some((_, path)) = self.operation_log_display() {
                        self.push_log(LogLevel::Info, format!("操作日志：{path}"));
                    }
                    self.push_log(LogLevel::Info, format!("开始{name}，共 {steps} 个步骤"));
                }
                ExecutionEvent::StepStarted {
                    index,
                    total,
                    label,
                    command,
                } => {
                    self.status_text = format!("[{index}/{total}] {label}");
                    self.push_log(LogLevel::Command, format!("$ {command}"));
                }
                ExecutionEvent::Output { text, stderr } => {
                    let lower = text.to_ascii_lowercase();
                    let level = if stderr || lower.contains("fail") || lower.contains("error") {
                        LogLevel::Error
                    } else if lower.contains("success") || lower.ends_with(" ok.") {
                        LogLevel::Success
                    } else {
                        LogLevel::Info
                    };
                    self.push_log(level, text);
                }
                ExecutionEvent::Progress(value) => self.progress = value,
                ExecutionEvent::TaskFinished {
                    name,
                    success,
                    cancelled,
                    message,
                } => {
                    self.progress = if success { 1.0 } else { self.progress };
                    self.status_text = if success {
                        format!("{name}完成")
                    } else if cancelled {
                        format!("{name}已中止")
                    } else {
                        format!("{name}未完成")
                    };
                    let level = if success {
                        LogLevel::Success
                    } else if cancelled {
                        LogLevel::Warning
                    } else {
                        LogLevel::Error
                    };
                    self.push_log(level, format!("{name}：{message}"));
                    if !success && !cancelled {
                        self.show_error_toast(format!("{name}失败"), message);
                    }
                }
            }
        }
    }
}
