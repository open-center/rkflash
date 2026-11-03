use super::FlashApplication;
use rkflash_api::{ExecutionEvent, TaskApi};

impl TaskApi for FlashApplication {
    fn drain_task_events(&self) -> Vec<ExecutionEvent> {
        self.executor.drain_events()
    }

    fn is_task_running(&self) -> bool {
        self.executor.is_active()
    }

    fn cancel_task(&self) {
        self.executor.cancel();
    }
}
