use crate::domain::{DeviceSnapshot, ExecutionEvent, ExecutionPlan};
use std::path::PathBuf;

pub(crate) trait OperationExecutor {
    fn is_active(&self) -> bool;
    fn start(&self, plan: ExecutionPlan) -> Result<(), String>;
    fn cancel(&self);
    fn drain_events(&self) -> Vec<ExecutionEvent>;
}

pub(crate) trait DeviceMonitor {
    fn latest_device_snapshot(&self) -> Option<DeviceSnapshot>;
}

pub(crate) trait ToolLocator {
    fn tool_path(&self) -> Option<PathBuf>;
    fn set_tool_path(&self, path: Option<PathBuf>);
}

pub(crate) trait OperationLog {
    fn tool_log_directory(&self) -> PathBuf;
    fn set_tool_log_directory(&self, path: PathBuf) -> Result<(), String>;
    fn begin_operation_log(&self, operation: &str) -> Result<PathBuf, String>;
    fn finish_operation_log(&self);
    fn operation_log_path(&self) -> Option<PathBuf>;
    fn append_operation_log(&self, level: &str, text: &str) -> Result<(), String>;
}
