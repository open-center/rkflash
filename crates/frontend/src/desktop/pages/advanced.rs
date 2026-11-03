use super::super::*;
use crate::desktop::actions::UNSUPPORTED_HINT;

/// 未设置 RKFLASH_LOG_DIR 时的默认日志目录（系统临时目录下）。
fn default_log_directory() -> String {
    std::env::temp_dir()
        .join("rkflash")
        .join("log")
        .display()
        .to_string()
}

impl RkFlashApp {
    pub(in crate::desktop) fn render_advanced(&mut self, ui: &mut egui::Ui) {
        self.page_heading(
            ui,
            "03",
            "高级功能",
            "面向调试、救砖与产线定位的底层设备操作",
        );

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let layout = LayoutProfile::from_width(ui.available_width());
                if layout.split_content {
                    ui.columns(2, |columns| {
                        self.render_device_queries(&mut columns[0]);
                        self.render_device_controls(&mut columns[1]);
                    });
                } else {
                    self.render_device_queries(ui);
                    ui.add_space(layout.card_gap);
                    self.render_device_controls(ui);
                }

                ui.add_space(layout.card_gap);
                if layout.split_content {
                    ui.columns(2, |columns| {
                        self.render_loader_upgrade(&mut columns[0]);
                        self.render_lba_tools(&mut columns[1]);
                    });
                } else {
                    self.render_loader_upgrade(ui);
                    ui.add_space(layout.card_gap);
                    self.render_lba_tools(ui);
                }

                ui.add_space(layout.card_gap);
                card().show(ui, |ui| {
                    section_title(ui, "串口日志导出", "通过烧录工具将设备日志保存到本地文件");
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [ui.available_width() - 118.0, 34.0],
                            egui::TextEdit::singleline(&mut self.output_path)
                                .hint_text("输出文件路径"),
                        );
                        if ui.button("选择输出").clicked() {
                            let spec = FileDialogSpec::save_file(
                                "选择串口日志输出文件",
                                "日志文件",
                                &["log", "txt"],
                                "rockchip-serial.log",
                            );
                            self.spawn_file_dialog(spec, FileDialogTarget::SerialLogOutput);
                        }
                        if ui
                            .add_enabled(
                                !self.is_task_running()
                                    && !self.output_path.trim().is_empty()
                                    && self.tool.serial_log,
                                egui::Button::new("开始导出"),
                            )
                            .on_disabled_hover_text(UNSUPPORTED_HINT)
                            .clicked()
                        {
                            self.export_serial_log();
                        }
                    });
                });

                ui.add_space(12.0);
                card().show(ui, |ui| {
                    section_title(ui, "工具运行日志", "每次操作单独归档，文件名使用本地时间戳");
                    let active = self.is_task_running();
                    let operation_log = self.operation_log_display();

                    inset().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            status_badge(
                                ui,
                                if active {
                                    "正在记录"
                                } else {
                                    "归档就绪"
                                },
                                if active {
                                    theme::ACCENT
                                } else {
                                    theme::SUCCESS
                                },
                                theme::ACCENT_SOFT,
                            );
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(
                                        operation_log
                                            .as_ref()
                                            .map(|(name, _)| name.as_str())
                                            .unwrap_or("下一次操作将创建新日志"),
                                    )
                                    .monospace()
                                    .size(12.0)
                                    .color(theme::TEXT),
                                )
                                .on_hover_text(
                                    operation_log
                                        .as_ref()
                                        .map(|(_, path)| path.as_str())
                                        .unwrap_or("尚未执行操作"),
                                );
                                ui.label(
                                    RichText::new("命令、标准输出、错误信息和最终结果都会保留")
                                        .small()
                                        .color(theme::MUTED_DARK),
                                );
                            });
                        });
                    });
                    ui.add_space(10.0);
                    ui.label(RichText::new("日志目录").small().color(theme::MUTED));
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [ui.available_width() - 238.0, 34.0],
                            egui::TextEdit::singleline(&mut self.tool_log_directory)
                                .hint_text(format!("日志目录，例如 {}", default_log_directory())),
                        );
                        if ui
                            .add_enabled(!active, egui::Button::new("选择目录"))
                            .on_hover_text(if active {
                                "当前任务结束后可以修改日志目录"
                            } else {
                                "选择一个专用日志目录"
                            })
                            .clicked()
                        {
                            let current = Path::new(self.tool_log_directory.trim());
                            let directory = if current.is_dir() {
                                Some(current.to_path_buf())
                            } else {
                                None
                            };
                            self.spawn_file_dialog(
                                FileDialogSpec {
                                    title: Some("选择工具日志目录".to_owned()),
                                    filter_name: "",
                                    filters: &[],
                                    file_name: None,
                                    directory,
                                    kind: crate::desktop::file_dialog::FileDialogKind::PickFolder,
                                },
                                FileDialogTarget::ToolLogDirectory,
                            );
                        }
                        if ui
                            .add_enabled(!active, egui::Button::new("应用位置"))
                            .on_hover_text(if active {
                                "当前任务结束后可以修改日志目录"
                            } else {
                                "立即使用此目录保存后续操作日志"
                            })
                            .clicked()
                        {
                            let path = PathBuf::from(self.tool_log_directory.trim());
                            self.set_tool_log_directory(path);
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("默认目录")
                                .monospace()
                                .size(10.0)
                                .color(theme::MUTED_DARK),
                        );
                        ui.label(
                            RichText::new(default_log_directory())
                                .monospace()
                                .size(11.0)
                                .color(theme::MUTED),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new("可通过界面或 RKFLASH_LOG_DIR 覆盖")
                                .small()
                                .color(theme::MUTED_DARK),
                        );
                    });
                });
            });
    }

    fn render_device_queries(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            section_title(ui, "设备信息", "只读命令，不修改设备内容");
            button_grid(ui, |ui| {
                self.query_button(ui, "读取 Flash ID", DeviceQuery::FlashId);
                self.query_button(ui, "读取 Flash 信息", DeviceQuery::FlashInfo);
                self.query_button(ui, "读取 Chip 信息", DeviceQuery::ChipInfo);
                self.query_button(ui, "读取 Capability", DeviceQuery::Capability);
                self.query_button(ui, "读取分区表", DeviceQuery::PartitionTable);
                self.query_button(ui, "读取 USB 序列号", DeviceQuery::UsbSerialNumber);
                self.query_button(ui, "检测安全模式", DeviceQuery::SecureMode);
                self.query_button(ui, "测试设备", DeviceQuery::TestDevice);
            });
        });
    }

    fn render_device_controls(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            section_title(ui, "设备控制", "操作期间设备会重新枚举或改变状态");
            button_grid(ui, |ui| {
                self.control_button(ui, "重启设备", DeviceControl::Reboot, false);
                self.control_button(ui, "切换存储", DeviceControl::SwitchStorage, true);
                self.control_button(ui, "清空序列号", DeviceControl::ClearSerial, true);
                self.control_button(ui, "重置 USB 管道", DeviceControl::ResetUsbPipe, false);
            });
        });
    }

    fn render_loader_upgrade(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            section_title(
                ui,
                "Loader 升级",
                "仅升级当前设备的 Loader，不主动切换设备模式",
            );
            if path_picker(ui, &mut self.loader_path, "MiniLoaderAll.bin") {
                let spec =
                    FileDialogSpec::pick_file("选择 Loader 文件", "Loader 镜像", &["bin", "img"]);
                self.spawn_file_dialog(spec, FileDialogTarget::LoaderBinary);
            }
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("loader_storage")
                    .selected_text(["自动", "eMMC", "SPI NOR", "SPI NAND"][self.loader_storage])
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.loader_storage, 0, "自动");
                        ui.selectable_value(&mut self.loader_storage, 1, "eMMC");
                        ui.selectable_value(&mut self.loader_storage, 2, "SPI NOR");
                        if self.tool.loader_storages.contains(&StorageType::SpiNand) {
                            ui.selectable_value(&mut self.loader_storage, 3, "SPI NAND");
                        }
                    });
                if ui
                    .add_enabled(
                        !self.is_task_running() && !self.loader_path.trim().is_empty(),
                        egui::Button::new(RichText::new("升级 Loader").color(theme::WARNING)),
                    )
                    .clicked()
                {
                    let storage = [
                        StorageType::Auto,
                        StorageType::Emmc,
                        StorageType::SpiNor,
                        StorageType::SpiNand,
                    ][self.loader_storage];
                    self.request_upgrade_loader(storage);
                }
            });
        });
    }

    fn render_lba_tools(&mut self, ui: &mut egui::Ui) {
        card().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            section_title(ui, "LBA 扇区操作", "地址和长度单位均为 512 字节扇区");
            ui.horizontal_wrapped(|ui| {
                field(ui, "起始扇区", &mut self.lba_start, 124.0);
                field(ui, "扇区数量", &mut self.lba_count, 124.0);
            });
            if path_picker(ui, &mut self.lba_image_path, "读取输出或写入镜像路径") {
                self.spawn_file_dialog(
                    FileDialogSpec::pick_file("选择 LBA 镜像", "镜像文件", &["img", "bin"]),
                    FileDialogTarget::LbaImage,
                );
            }
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(!self.is_task_running(), egui::Button::new("读取 LBA"))
                    .clicked()
                {
                    if self.lba_image_path.trim().is_empty() {
                        // 路径为空时先选择输出文件；选择完成后自动开始读取。
                        self.spawn_file_dialog(
                            FileDialogSpec {
                                title: Some("选择 LBA 读取输出文件".to_owned()),
                                filter_name: "镜像文件",
                                filters: &["img", "bin"],
                                file_name: Some("flash-export.img".to_owned()),
                                directory: None,
                                kind: crate::desktop::file_dialog::FileDialogKind::SaveFile,
                            },
                            FileDialogTarget::LbaExport,
                        );
                    } else {
                        self.read_lba(PathBuf::from(&self.lba_image_path));
                    }
                }
                if ui
                    .add_enabled(
                        !self.is_task_running() && !self.lba_image_path.trim().is_empty(),
                        egui::Button::new(RichText::new("写入 LBA").color(theme::WARNING))
                            .fill(theme::WARNING_SOFT),
                    )
                    .clicked()
                {
                    self.request_write_lba();
                }
                if ui
                    .add_enabled(!self.is_task_running(), danger_button("擦除 LBA"))
                    .clicked()
                {
                    self.request_erase_lba();
                }
            });
        });
    }

    fn query_button(&mut self, ui: &mut egui::Ui, label: &str, query: DeviceQuery) {
        let supported = self.tool.queries.contains(&query);
        if ui
            .add_enabled(
                supported && !self.is_task_running(),
                egui::Button::new(RichText::new(label).color(theme::TEXT))
                    .fill(theme::PANEL_RAISED)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER)),
            )
            .on_disabled_hover_text(UNSUPPORTED_HINT)
            .clicked()
        {
            self.query_device(query);
        }
    }

    fn control_button(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        control: DeviceControl,
        confirm: bool,
    ) {
        let supported = self.tool.controls.contains(&control);
        if ui
            .add_enabled(
                supported && !self.is_task_running(),
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
            self.request_control_device(control, confirm);
        }
    }
}
