use super::super::*;

impl RkFlashApp {
    pub(in crate::desktop) fn render_download(&mut self, ui: &mut egui::Ui) {
        let layout = LayoutProfile::from_width(ui.available_width());
        if layout.split_content {
            self.render_download_content(ui);
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| self.render_download_content(ui));
        }
    }

    fn render_download_content(&mut self, ui: &mut egui::Ui) {
        self.page_heading(ui, "01", "下载镜像", "编排分区镜像并按顺序写入目标设备");
        self.render_download_toolbar(ui);
        ui.add_space(12.0);

        let enabled = self.images.iter().filter(|entry| entry.enabled).count();
        let ready = self
            .images
            .iter()
            .filter(|entry| entry.enabled && !entry.path.trim().is_empty())
            .count();
        let layout = LayoutProfile::from_width(ui.available_width());
        let mut browse_index = None;
        let mut remove_index = None;

        if layout.split_content {
            let available_width = ui.available_width();
            let sidebar_width = 310.0;
            let queue_width = available_width - sidebar_width - layout.card_gap;

            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(queue_width, 330.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        (browse_index, remove_index) = self.render_image_queue(ui, 232.0);
                    },
                );
                ui.add_space(layout.card_gap);
                ui.allocate_ui_with_layout(
                    egui::vec2(sidebar_width, 330.0),
                    Layout::top_down(Align::Min),
                    |ui| self.render_download_controls(ui, enabled, ready, false),
                );
            });
        } else {
            self.render_download_controls(ui, enabled, ready, true);
            ui.add_space(layout.card_gap);
            (browse_index, remove_index) = self.render_image_queue(ui, 300.0);
        }

        self.handle_image_row_actions(browse_index, remove_index);
        ui.add_space(2.0);
    }

    fn render_download_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("配置")
                    .monospace()
                    .size(10.0)
                    .color(theme::MUTED_DARK),
            );
            if ui.button("加载 parameter").clicked() {
                self.load_parameter_file();
            }
            if ui.button("导入配置").clicked() {
                self.import_workspace();
            }
            if ui.button("导出配置").clicked() {
                self.export_workspace();
            }
            if ui.button("设备分区表").clicked() {
                self.run_query(DeviceQuery::PartitionTable);
            }
            ui.separator();
            if ui.button("恢复默认").clicked() {
                self.images = self.application.default_images();
                self.selected_image = None;
            }
            if ui
                .add(egui::Button::new(
                    RichText::new("清空路径").color(theme::DANGER),
                ))
                .clicked()
            {
                for image in &mut self.images {
                    image.path.clear();
                    image.enabled = false;
                }
            }
        });
    }

    fn render_image_queue(
        &mut self,
        ui: &mut egui::Ui,
        table_height: f32,
    ) -> (Option<usize>, Option<usize>) {
        let mut browse_index = None;
        let mut remove_index = None;

        card().show(ui, |ui| {
            section_title(
                ui,
                "镜像写入队列",
                "由上至下执行；可单独启用、排序或删除下载项",
            );
            let path_width = (ui.available_width() - 475.0).max(230.0);
            egui::ScrollArea::both()
                .max_height(table_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("image_table")
                        .striped(true)
                        .num_columns(6)
                        .min_col_width(56.0)
                        .spacing([10.0, 7.0])
                        .show(ui, |ui| {
                            table_header(ui, "启用");
                            table_header(ui, "地址 / 扇区");
                            table_header(ui, "分区名");
                            table_header(ui, "镜像文件");
                            ui.label("");
                            ui.label("");
                            ui.end_row();

                            for index in 0..self.images.len() {
                                ui.checkbox(&mut self.images[index].enabled, "");
                                ui.add_sized(
                                    [112.0, 30.0],
                                    egui::TextEdit::singleline(&mut self.images[index].address)
                                        .font(egui::TextStyle::Monospace),
                                );
                                let selected = self.selected_image == Some(index);
                                if ui
                                    .add_sized(
                                        [104.0, 30.0],
                                        egui::SelectableLabel::new(
                                            selected,
                                            &self.images[index].name,
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.selected_image = Some(index);
                                }
                                ui.add_sized(
                                    [path_width, 30.0],
                                    egui::TextEdit::singleline(&mut self.images[index].path),
                                );
                                if ui.small_button("选择").clicked() {
                                    browse_index = Some(index);
                                }
                                if ui.small_button("删除").clicked() {
                                    remove_index = Some(index);
                                }
                                ui.end_row();
                            }
                        });
                });
        });

        (browse_index, remove_index)
    }

    fn render_download_controls(
        &mut self,
        ui: &mut egui::Ui,
        enabled: usize,
        ready: usize,
        compact: bool,
    ) {
        card().show(ui, |ui| {
            if compact {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("下载控制台")
                                .strong()
                                .size(16.0)
                                .color(theme::TEXT),
                        );
                        ui.label(
                            RichText::new("确认队列状态后开始写入")
                                .small()
                                .color(theme::MUTED),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.render_start_download(ui, 170.0);
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    metric(ui, "下载项", self.images.len().to_string());
                    ui.separator();
                    metric(ui, "已启用", enabled.to_string());
                    ui.separator();
                    metric(ui, "路径就绪", format!("{ready}/{enabled}"));
                    ui.separator();
                    self.render_queue_management(ui);
                    ui.checkbox(&mut self.reset_after_download, "完成后重启");
                });
                return;
            }

            section_title(ui, "下载控制台", "确认队列状态后开始写入");

            ui.columns(3, |columns| {
                metric(&mut columns[0], "下载项", self.images.len().to_string());
                metric(&mut columns[1], "已启用", enabled.to_string());
                metric(&mut columns[2], "路径就绪", format!("{ready}/{enabled}"));
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(
                RichText::new("队列管理")
                    .monospace()
                    .size(10.0)
                    .color(theme::MUTED_DARK),
            );
            ui.horizontal_wrapped(|ui| {
                self.render_queue_management(ui);
            });

            ui.add_space(6.0);
            ui.checkbox(&mut self.reset_after_download, "完成后重启设备");
            ui.add_space(5.0);
            ui.label(
                RichText::new(if enabled == ready && enabled > 0 {
                    "路径已就绪，可以开始设备校验"
                } else {
                    "请补齐所有已启用项的镜像路径"
                })
                .small()
                .color(if enabled == ready && enabled > 0 {
                    theme::SUCCESS
                } else {
                    theme::WARNING
                }),
            );
            ui.add_space(8.0);
            self.render_start_download(ui, ui.available_width());
        });
    }

    fn render_queue_management(&mut self, ui: &mut egui::Ui) {
        if ui.button("+ 添加").clicked() {
            self.images
                .push(ImageEntry::new(false, "0x00000000", "new_partition"));
            self.selected_image = Some(self.images.len() - 1);
        }
        if ui
            .add_enabled(self.can_move_selected(-1), egui::Button::new("上移"))
            .clicked()
        {
            self.move_selected(-1);
        }
        if ui
            .add_enabled(self.can_move_selected(1), egui::Button::new("下移"))
            .clicked()
        {
            self.move_selected(1);
        }
    }

    fn render_start_download(&mut self, ui: &mut egui::Ui, width: f32) {
        if ui
            .add_enabled(
                !self.application.is_active(),
                egui::Button::new(RichText::new("开始下载").strong().color(Color32::WHITE))
                    .fill(theme::ACCENT_DARK)
                    .stroke(Stroke::new(1.0_f32, theme::ACCENT))
                    .min_size(egui::vec2(width, 40.0)),
            )
            .clicked()
        {
            self.confirm(
                "确认写入镜像",
                "将按当前顺序写入所有已勾选项。错误的镜像或地址可能导致设备无法启动。",
                true,
                FlashOperation::DownloadImages {
                    images: self.images.clone(),
                    reset_after: self.reset_after_download,
                },
            );
        }
    }

    fn handle_image_row_actions(
        &mut self,
        browse_index: Option<usize>,
        remove_index: Option<usize>,
    ) {
        if let Some(index) = browse_index {
            self.spawn_file_dialog(
                FileDialogSpec::pick_file("选择镜像文件", "镜像文件", &["img", "bin", "txt"]),
                FileDialogTarget::DownloadImage(index),
            );
        }
        if let Some(index) = remove_index {
            self.images.remove(index);
            self.selected_image = None;
        }
    }
}

fn table_header(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .monospace()
            .size(10.0)
            .color(theme::MUTED_DARK),
    );
}
