//! 配置导入导出 ↔ `WorkspaceApi`。

use super::super::*;

impl RkFlashApp {
    pub(in crate::desktop) fn pick_workspace_export(&mut self) {
        self.spawn_file_dialog(
            FileDialogSpec::save_file(
                "导出 RK Flash 配置",
                "RK Flash 配置",
                &["json"],
                "flash-layout.rkflash.json",
            ),
            FileDialogTarget::WorkspaceExport,
        );
    }

    pub(in crate::desktop) fn pick_workspace_import(&mut self) {
        self.spawn_file_dialog(
            FileDialogSpec::pick_file("导入 RK Flash 配置", "RK Flash 配置", &["json"]),
            FileDialogTarget::WorkspaceImport,
        );
    }

    pub(in crate::desktop) fn save_workspace(&mut self, path: PathBuf) {
        let config = WorkspaceConfig {
            format_version: WORKSPACE_FORMAT_VERSION,
            images: self.images.clone(),
            firmware_path: self.firmware_path.clone(),
            loader_path: self.loader_path.clone(),
            reset_after_download: self.reset_after_download,
            no_reset_after_upgrade: self.no_reset_after_upgrade,
        };
        match self.application.save_workspace(&path, &config) {
            Ok(()) => self.push_log(LogLevel::Success, format!("配置已导出：{}", path.display())),
            Err(error) => self.push_log(LogLevel::Error, error),
        }
    }

    pub(in crate::desktop) fn load_workspace(&mut self, path: PathBuf) {
        match self.application.load_workspace(&path) {
            Ok(config) => {
                self.images = config.images;
                self.firmware_path = config.firmware_path;
                self.loader_path = config.loader_path;
                self.reset_after_download = config.reset_after_download;
                self.no_reset_after_upgrade = config.no_reset_after_upgrade;
                self.selected_image = None;
                self.push_log(LogLevel::Success, format!("配置已导入：{}", path.display()));
            }
            Err(error) => self.push_log(LogLevel::Error, error),
        }
    }
}
