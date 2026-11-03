//! 任务状态：前端可观察的执行事件，不暴露后端执行计划与适配器细节。

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

pub trait TaskApi {
    /// 取走自上次调用以来的全部执行事件。
    fn drain_task_events(&self) -> Vec<ExecutionEvent>;
    fn is_task_running(&self) -> bool;
    fn cancel_task(&self);
}
