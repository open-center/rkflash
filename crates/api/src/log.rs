//! 操作日志：每次任务一个日志文件，界面日志同步写入。

use std::path::PathBuf;

pub trait LogApi {
    fn operation_log_path(&self) -> Option<PathBuf>;
    fn append_operation_log(&self, level: &str, text: &str) -> Result<(), String>;
}
