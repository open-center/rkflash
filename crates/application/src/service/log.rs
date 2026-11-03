use super::FlashApplication;
use rkflash_api::LogApi;
use std::path::PathBuf;

impl LogApi for FlashApplication {
    fn operation_log_path(&self) -> Option<PathBuf> {
        self.operation_log.operation_log_path()
    }

    fn append_operation_log(&self, level: &str, text: &str) -> Result<(), String> {
        self.operation_log.append_operation_log(level, text)
    }
}
