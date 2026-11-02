//! 文件选择、工作区管理与类型化操作触发。

use super::*;

/// 按钮因任务运行或平台不支持而禁用时的提示。
pub(super) const UNSUPPORTED_HINT: &str = "任务运行中，或当前平台的烧录工具不支持该操作";

impl RkFlashApp {
    pub(super) fn apply_tool_log_directory(&mut self, path: PathBuf) {
        match self.application.set_tool_log_directory(path) {
            Ok(()) => {
                let active_path = self.application.tool_log_directory();
                self.tool_log_directory = active_path.display().to_string();
                self.push_log(
                    LogLevel::Success,
                    format!("工具日志目录已切换至：{}", active_path.display()),
                );
            }
            Err(error) => self.report_error("无法切换工具日志目录", error),
        }
    }

    pub(super) fn pick_upgrade_tool(&mut self) {
        self.spawn_file_dialog(
            FileDialogSpec::pick_file(
                &format!("选择 {}", self.application.tool_name()),
                "可执行文件",
                &[],
            ),
            FileDialogTarget::ToolExecutable,
        );
    }

    pub(super) fn load_parameter_file(&mut self) {
        self.spawn_file_dialog(
            FileDialogSpec::pick_file(
                "选择 parameter.txt",
                "Rockchip parameter",
                &["txt", "parameter"],
            ),
            FileDialogTarget::ParameterFile,
        );
    }

    pub(super) fn export_workspace(&mut self) {
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

    pub(super) fn import_workspace(&mut self) {
        self.spawn_file_dialog(
            FileDialogSpec::pick_file("导入 RK Flash 配置", "RK Flash 配置", &["json"]),
            FileDialogTarget::WorkspaceImport,
        );
    }

    /// 后台文件对话框完成后的结果分发；只在用户确实选择了路径时调用。
    pub(super) fn apply_file_dialog_result(&mut self, target: &FileDialogTarget, path: PathBuf) {
        match target {
            FileDialogTarget::ToolExecutable => {
                match self.application.set_tool_path(path.clone()) {
                    Ok(()) => self.push_log(
                        LogLevel::Success,
                        format!("已设置工具路径：{}", path.display()),
                    ),
                    Err(error) => self.push_log(LogLevel::Error, error),
                }
            }
            FileDialogTarget::ParameterFile => self.apply_parameter_file(path),
            FileDialogTarget::WorkspaceExport => self.export_workspace_to(path),
            FileDialogTarget::WorkspaceImport => self.import_workspace_from(path),
            FileDialogTarget::FirmwareImage => {
                self.firmware_path = path.display().to_string();
            }
            FileDialogTarget::DownloadImage(index) => {
                if let Some(entry) = self.images.get_mut(*index) {
                    entry.path = path.display().to_string();
                    entry.enabled = true;
                }
            }
            FileDialogTarget::LoaderBinary => {
                self.loader_path = path.display().to_string();
            }
            FileDialogTarget::LbaImage => {
                self.lba_image_path = path.display().to_string();
            }
            FileDialogTarget::LbaExport => {
                self.lba_image_path = path.display().to_string();
                self.run_operation(FlashOperation::ReadLba {
                    start: self.lba_start.clone(),
                    count: self.lba_count.clone(),
                    output: path,
                });
            }
            FileDialogTarget::SerialLogOutput => {
                self.output_path = path.display().to_string();
            }
            FileDialogTarget::ToolLogDirectory => self.apply_tool_log_directory(path),
        }
    }

    fn apply_parameter_file(&mut self, path: PathBuf) {
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

    fn export_workspace_to(&mut self, path: PathBuf) {
        match self
            .application
            .save_workspace(&path, &self.workspace_config())
        {
            Ok(()) => self.push_log(LogLevel::Success, format!("配置已导出：{}", path.display())),
            Err(error) => self.push_log(LogLevel::Error, error),
        }
    }

    fn import_workspace_from(&mut self, path: PathBuf) {
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

    fn workspace_config(&self) -> WorkspaceConfig {
        WorkspaceConfig {
            format_version: WORKSPACE_FORMAT_VERSION,
            images: self.images.clone(),
            firmware_path: self.firmware_path.clone(),
            loader_path: self.loader_path.clone(),
            reset_after_download: self.reset_after_download,
            no_reset_after_upgrade: self.no_reset_after_upgrade,
        }
    }

    pub(super) fn can_move_selected(&self, offset: isize) -> bool {
        self.selected_image.is_some_and(|index| {
            let target = index as isize + offset;
            target >= 0 && target < self.images.len() as isize
        })
    }

    pub(super) fn move_selected(&mut self, offset: isize) {
        let Some(index) = self.selected_image else {
            return;
        };
        let target = (index as isize + offset) as usize;
        self.images.swap(index, target);
        self.selected_image = Some(target);
    }

    pub(super) fn firmware_kind(&self) -> String {
        if self.firmware_path.trim().is_empty() {
            "尚未选择".to_owned()
        } else if self.firmware_path.to_ascii_lowercase().ends_with(".img") {
            "Rockchip IMG".to_owned()
        } else {
            "未知格式".to_owned()
        }
    }

    pub(super) fn query_button(&mut self, ui: &mut egui::Ui, label: &str, query: DeviceQuery) {
        let supported = self.application.supports(&FlashOperation::Query { query });
        if ui
            .add_enabled(
                supported && !self.application.is_active(),
                egui::Button::new(RichText::new(label).color(theme::TEXT))
                    .fill(theme::PANEL_RAISED)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER)),
            )
            .on_disabled_hover_text(UNSUPPORTED_HINT)
            .clicked()
        {
            self.run_query(query);
        }
    }

    pub(super) fn run_query(&mut self, query: DeviceQuery) {
        self.run_operation(FlashOperation::Query { query });
    }

    pub(super) fn control_button(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        control: DeviceControl,
        confirm: bool,
    ) {
        let supported = self.application.supports(&FlashOperation::Control(control));
        if ui
            .add_enabled(
                supported && !self.application.is_active(),
                egui::Button::new(RichText::new(label).color(if confirm {
                    theme::WARNING
                } else {
                    theme::TEXT
                }))
                .fill(if confirm {
                    theme::WARNING_SOFT
                } else {
                    theme::PANEL_RAISED
                })
                .stroke(Stroke::new(
                    1.0_f32,
                    if confirm {
                        theme::WARNING
                    } else {
                        theme::BORDER
                    },
                )),
            )
            .on_disabled_hover_text(UNSUPPORTED_HINT)
            .clicked()
        {
            let operation = FlashOperation::Control(control);
            if confirm {
                self.confirm(
                    control.name(),
                    "此操作会改变设备当前状态，是否继续？",
                    false,
                    operation,
                );
            } else {
                self.run_operation(operation);
            }
        }
    }

    pub(super) fn confirm(
        &mut self,
        title: impl Into<String>,
        detail: impl Into<String>,
        danger: bool,
        operation: FlashOperation,
    ) {
        if let Err(error) = self.application.validate(&operation) {
            self.report_error("操作校验失败", error);
            return;
        }
        self.pending_action = Some(PendingAction {
            title: title.into(),
            detail: detail.into(),
            danger,
            operation,
        });
    }

    pub(super) fn run_operation(&mut self, operation: FlashOperation) {
        if let Err(error) = self.application.execute(operation) {
            self.report_error("无法开始操作", error);
        }
    }
}
