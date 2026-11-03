//! 下载镜像页 ↔ `DownloadApi`。

use super::super::*;

impl RkFlashApp {
    pub(in crate::desktop) fn restore_default_images(&mut self) {
        self.images = self.application.default_images();
        self.selected_image = None;
    }

    pub(in crate::desktop) fn pick_parameter_file(&mut self) {
        self.spawn_file_dialog(
            FileDialogSpec::pick_file(
                "选择 parameter.txt",
                "Rockchip parameter",
                &["txt", "parameter"],
            ),
            FileDialogTarget::ParameterFile,
        );
    }

    pub(in crate::desktop) fn import_parameter(&mut self, path: PathBuf) {
        match self.application.import_parameter(&path) {
            Ok(entries) => {
                self.images = entries;
                self.selected_image = None;
                let matched = self
                    .images
                    .iter()
                    .filter(|entry| !entry.path.trim().is_empty())
                    .count();
                self.push_log(
                    LogLevel::Success,
                    format!(
                        "已从 {} 加载 {} 个分区项，自动匹配 {} 个同目录固件",
                        path.display(),
                        self.images.len(),
                        matched,
                    ),
                );
                let missing = self
                    .images
                    .iter()
                    .filter(|entry| entry.path.trim().is_empty())
                    .map(|entry| entry.name.clone())
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    self.push_log(
                        LogLevel::Warning,
                        format!(
                            "同目录未找到以下分区固件，已保持未勾选：{}",
                            missing.join("、")
                        ),
                    );
                }
            }
            Err(error) => self.push_log(LogLevel::Error, error),
        }
    }

    pub(in crate::desktop) fn read_partition_table(&mut self) {
        let result = self.application.read_partition_table();
        self.report_start(result);
    }

    pub(in crate::desktop) fn request_download(&mut self) {
        let request = DownloadRequest {
            images: self.images.clone(),
            reset_after: self.reset_after_download,
        };
        self.ask_confirmation(
            self.application.check_download(&request),
            "确认写入镜像",
            "将按当前顺序写入所有已勾选项。错误的镜像或地址可能导致设备无法启动。",
            true,
            ConfirmedAction::Download(request),
        );
    }

    pub(super) fn download_images(&mut self, request: DownloadRequest) {
        let result = self.application.download_images(request);
        self.report_start(result);
    }
}
