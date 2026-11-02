use crate::domain::{CommandSpec, DeviceSnapshot, ExecutionPlan, FlashOperation};
use rkflash_config::ImageEntry;

/// 只在 UI 线程使用，不要求跨线程共享；`ToolProtocol` 才是后台线程依赖的端口。
pub(crate) trait FlashPlanner {
    fn plan(&self, operation: &FlashOperation) -> Result<ExecutionPlan, String>;
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
}
