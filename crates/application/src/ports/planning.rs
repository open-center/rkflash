use crate::domain::{CommandSpec, DeviceSnapshot, ExecutionPlan, FlashOperation, ToolInfo};
use rkflash_config::ImageEntry;

/// 只在 UI 线程使用，不要求跨线程共享；`ToolProtocol` 才是后台线程依赖的端口。
pub trait FlashPlanner {
    fn plan(&self, operation: &FlashOperation) -> Result<ExecutionPlan, String>;
    /// 外部工具的名称与能力；应用层据此拒绝不支持的操作，前端据此禁用对应入口。
    fn tool_info(&self) -> ToolInfo;
}

/// 只在 UI 线程使用，不要求跨线程共享。
pub trait FlashProfile {
    fn default_images(&self) -> Vec<ImageEntry>;
    fn parse_parameter(&self, text: &str) -> Result<Vec<ImageEntry>, String>;
}

pub trait ToolProtocol: Send + Sync {
    fn device_scan_command(&self) -> CommandSpec;
    fn parse_device_list(&self, text: &str) -> DeviceSnapshot;
    fn parse_progress(&self, line: &str) -> Option<f32>;
    fn parse_failure(&self, line: &str) -> Option<String>;
    /// 原地刷新的进度行：用于更新进度条，但不写入任务日志。
    fn is_transient(&self, _line: &str) -> bool {
        false
    }
}
