//! 前端可观察的任务状态，不暴露后端执行计划与适配器细节。

use crate::DeviceSnapshot;

pub struct ApplicationUpdate {
    pub device_snapshot: Option<DeviceSnapshot>,
    pub events: Vec<ExecutionEvent>,
}

#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    TaskStarted {
        name: String,
        steps: usize,
    },
    StepStarted {
        index: usize,
        total: usize,
        label: String,
        command: String,
    },
    Output {
        text: String,
        stderr: bool,
    },
    Progress(f32),
    TaskFinished {
        name: String,
        success: bool,
        cancelled: bool,
        message: String,
    },
}
