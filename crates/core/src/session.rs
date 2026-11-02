//! 烧录工作流与外部工具之间的会话接口：工作流只通过它调用工具、报告进度和检查取消，
//! 不关心工具进程如何启动，因此可在任何平台和测试中复用。

use std::ffi::OsString;

/// 工作流中单次工具调用的结果；只有进程无法启动或任务被中止时才返回 `Err`。
#[derive(Debug, Clone, Default)]
pub struct ToolOutcome {
    pub success: bool,
    /// 工具在 stdout/stderr 中报告的第一条失败信息。
    pub failure: Option<String>,
    pub output: String,
}

impl ToolOutcome {
    /// 把失败的调用转换为带步骤名的错误，便于工作流用 `?` 直接终止。
    pub fn require(self, label: &str) -> Result<Self, String> {
        if self.success && self.failure.is_none() {
            return Ok(self);
        }
        Err(match self.failure {
            Some(failure) => format!("步骤“{label}”失败：{failure}"),
            None => format!("步骤“{label}”执行失败"),
        })
    }
}

/// 运行时提供给工作流的受控能力：调用外部工具、报告状态与进度、检查取消。
pub trait ToolSession {
    /// 执行一次外部工具；输出同步进入任务日志。
    fn run(&mut self, label: &str, args: &[OsString]) -> Result<ToolOutcome, String>;
    /// 不回显命令和输出的探测调用（例如轮询设备模式）。
    fn probe(&mut self, args: &[OsString]) -> Result<ToolOutcome, String>;
    fn status(&mut self, text: &str);
    /// 之后的工具进度映射到整体进度的 `[start, end]` 区间。
    fn progress_range(&mut self, start: f32, end: f32);
    fn progress(&mut self, value: f32);
    fn is_cancelled(&self) -> bool;
}

/// 需要根据设备状态和工具输出动态决定后续步骤的多步任务（例如整包升级）。
pub trait Workflow: Send {
    /// 用于界面显示 “[当前/总数]” 的步骤数估计。
    fn estimated_steps(&self) -> usize;
    fn run(self: Box<Self>, session: &mut dyn ToolSession) -> Result<String, String>;
}
