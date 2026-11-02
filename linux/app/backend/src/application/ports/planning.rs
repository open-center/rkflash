use crate::domain::{CommandSpec, DeviceSnapshot, ExecutionPlan, FlashOperation};
use rkflash_config::ImageEntry;

/// 只在 UI 线程使用，不要求跨线程共享；`ToolProtocol` 才是后台线程依赖的端口。
pub(crate) trait FlashPlanner {
    fn plan(&self, operation: &FlashOperation) -> Result<ExecutionPlan, String>;
    /// 当前外部工具是否支持该操作；前端据此禁用对应按钮。
    fn supports(&self, _operation: &FlashOperation) -> bool {
        true
    }
    /// 界面中显示的外部工具名称。
    fn tool_name(&self) -> &'static str;
}

/// 只在 UI 线程使用，不要求跨线程共享。
pub(crate) trait FlashProfile {
    fn default_images(&self) -> Vec<ImageEntry>;
    fn parse_parameter(&self, text: &str) -> Result<Vec<ImageEntry>, String>;
}

pub(crate) trait ToolProtocol: Send + Sync {
    fn device_scan_command(&self) -> CommandSpec;
    fn parse_device_list(&self, text: &str) -> DeviceSnapshot;
    fn parse_progress(&self, line: &str) -> Option<f32>;
    fn parse_failure(&self, line: &str) -> Option<String>;
    /// 原地刷新的进度行：用于更新进度条，但不写入任务日志。
    fn is_transient(&self, _line: &str) -> bool {
        false
    }
}
