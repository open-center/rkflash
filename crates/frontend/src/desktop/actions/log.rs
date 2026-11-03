//! 运行终端与操作日志 ↔ `LogApi`。

use super::super::*;

impl RkFlashApp {
    /// 写入界面日志，并同步追加到当前操作日志文件。
    pub(in crate::desktop) fn push_log(&mut self, level: LogLevel, text: impl Into<String>) {
        let text = text.into();
        let persistence_result = self.application.append_operation_log(level.label(), &text);
        let elapsed = self.session_started_at.elapsed();
        self.logs.push(LogEntry {
            level,
            text,
            elapsed,
        });
        if let Err(error) = persistence_result {
            if self.log_persistence_error.as_deref() != Some(error.as_str()) {
                self.logs.push(LogEntry {
                    level: LogLevel::Error,
                    text: format!("操作日志未能写入磁盘：{error}"),
                    elapsed: self.session_started_at.elapsed(),
                });
                self.log_persistence_error = Some(error);
            }
        }
        if self.logs.len() > 2_000 {
            self.logs.drain(..500);
        }
    }

    /// 当前操作日志的（文件名，完整路径）。
    pub(in crate::desktop) fn operation_log_display(&self) -> Option<(String, String)> {
        self.application.operation_log_path().map(|path| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            (name, path.display().to_string())
        })
    }
}
