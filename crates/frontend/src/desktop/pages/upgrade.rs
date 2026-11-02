use super::super::*;

/// 整包升级前用 `cs` 选择的目标存储；upgrade_tool 自动识别，不显示该选项。
const FIRMWARE_STORAGES: [(&str, StorageType); 3] = [
    ("eMMC", StorageType::Emmc),
    ("SPI NOR", StorageType::SpiNor),
    ("自动（Loader 默认存储）", StorageType::Auto),
];

impl RkFlashApp {
    fn firmware_storage_selectable(&self) -> bool {
        self.application.tool_name() == "rkdeveloptool"
    }

    pub(in crate::desktop) fn render_upgrade(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| self.render_upgrade_content(ui));
    }

    fn render_upgrade_content(&mut self, ui: &mut egui::Ui) {
        self.page_heading(
            ui,
            "02",
            "升级固件",
            "校验并部署 Rockchip 统一固件 update.img",
        );

        card().show(ui, |ui| {
            section_title(
                ui,
                "选择统一固件",
                &format!(
                    "固件将在任务开始时由 {} 完成格式校验",
                    self.application.tool_name()
                ),
            );
            ui.horizontal(|ui| {
                ui.add_sized(
                    [ui.available_width() - 100.0, 38.0],
                    egui::TextEdit::singleline(&mut self.firmware_path)
                        .hint_text("选择 update.img 或将绝对路径粘贴到这里"),
                );
                if ui.button("选择固件").clicked() {
                    self.spawn_file_dialog(
                        FileDialogSpec::pick_file("选择 update.img", "Rockchip 固件", &["img"]),
                        FileDialogTarget::FirmwareImage,
                    );
                }
            });

            ui.add_space(12.0);
            stat_strip(
                ui,
                &[
                    ("文件类型", self.firmware_kind()),
                    ("文件大小", self.cached_firmware_size()),
                    ("校验方式", "任务启动时校验".to_owned()),
                ],
            );
        });

        ui.add_space(14.0);
        card().show(ui, |ui| {
            section_title(ui, "升级流程", "设备必须处于 Maskrom 或 Loader 模式");
            inset().show(ui, |ui| {
                ui.columns(3, |columns| {
                    upgrade_step(&mut columns[0], "01", "连接", "识别唯一目标设备");
                    upgrade_step(&mut columns[1], "02", "校验", "确认固件格式可用");
                    upgrade_step(&mut columns[2], "03", "写入", "保持 USB 与供电稳定");
                });
            });
            ui.add_space(12.0);
            if self.firmware_storage_selectable() {
                ui.horizontal(|ui| {
                    ui.label("目标存储");
                    egui::ComboBox::from_id_salt("firmware_storage")
                        .selected_text(FIRMWARE_STORAGES[self.firmware_storage].0)
                        .show_ui(ui, |ui| {
                            for (index, (label, _)) in FIRMWARE_STORAGES.iter().enumerate() {
                                ui.selectable_value(&mut self.firmware_storage, index, *label);
                            }
                        });
                });
                ui.add_space(6.0);
            }
            ui.checkbox(
                &mut self.no_reset_after_upgrade,
                "升级完成后保持当前状态，不自动重启",
            );
            ui.add_space(14.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        !self.application.is_active() && !self.firmware_path.trim().is_empty(),
                        primary_button("开始升级", theme::ACCENT_DARK),
                    )
                    .clicked()
                {
                    self.confirm(
                        "确认升级统一固件",
                        "升级会重写设备分区。请确认固件型号与目标芯片匹配，并保持 USB 和电源稳定。",
                        true,
                        FlashOperation::UpgradeFirmware {
                            firmware: Path::new(&self.firmware_path).to_path_buf(),
                            no_reset: self.no_reset_after_upgrade,
                            storage: FIRMWARE_STORAGES[self.firmware_storage].1,
                        },
                    );
                }

                if ui
                    .add_enabled(
                        !self.application.is_active() && !self.firmware_path.trim().is_empty(),
                        danger_button("擦除 Flash"),
                    )
                    .clicked()
                {
                    self.confirm(
                        "确认擦除 Flash",
                        "此操作会清除目标存储上的全部数据，无法撤销。",
                        true,
                        FlashOperation::EraseFlash {
                            firmware: Path::new(&self.firmware_path).to_path_buf(),
                        },
                    );
                }

                ui.add_enabled(false, egui::Button::new("解包固件"))
                    .on_disabled_hover_text("升级时会自动解包并按固件清单写入各分区");
            });
        });
        ui.add_space(2.0);
    }
}

fn upgrade_step(ui: &mut egui::Ui, index: &str, title: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(index)
                .monospace()
                .strong()
                .color(theme::ACCENT),
        );
        ui.vertical(|ui| {
            ui.label(RichText::new(title).strong().color(theme::TEXT));
            ui.label(RichText::new(detail).small().color(theme::MUTED));
        });
    });
}
