//! 顶部导航、设备状态与运行终端外壳。

use super::*;

impl RkFlashApp {
    fn render_device_hover_tip(&self, response: egui::Response) {
        let Some(mode) = self.current_device_mode() else {
            return;
        };
        let guidance = mode.guidance();
        let (accent, icon) = match mode {
            DeviceMode::Maskrom => (theme::WARNING, "!"),
            DeviceMode::Loader => (theme::SUCCESS, "✓"),
        };

        response.on_hover_ui(|ui| {
            ui.set_max_width(340.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).strong().size(16.0).color(accent));
                ui.label(RichText::new(guidance.title).strong().color(theme::TEXT));
            });
            ui.add_space(4.0);
            ui.label(RichText::new(guidance.detail).color(theme::MUTED));
        });
    }

    pub(super) fn render_header(&mut self, context: &egui::Context) {
        let layout = LayoutProfile::from_width(context.available_rect().width());
        egui::TopBottomPanel::top("app_header")
            .exact_height(78.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER_SOFT))
                    .inner_margin(egui::Margin::symmetric(layout.content_margin, 13)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    let (mark, _) =
                        ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::hover());
                    ui.painter().rect_filled(mark, 5.0, theme::ACCENT_DARK);
                    ui.painter().rect_stroke(
                        mark.shrink(4.0),
                        3.0,
                        Stroke::new(1.0_f32, theme::ACCENT),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        mark.center(),
                        egui::Align2::CENTER_CENTER,
                        "RK",
                        egui::FontId::monospace(15.0),
                        Color32::WHITE,
                    );
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("RK FLASH STUDIO")
                                .size(16.0)
                                .strong()
                                .color(theme::TEXT),
                        );
                        if !layout.compact_header {
                            ui.label(
                                RichText::new("DEVICE OPERATIONS CONSOLE")
                                    .monospace()
                                    .size(10.0)
                                    .color(theme::MUTED_DARK),
                            );
                        }
                    });

                    ui.add_space(if layout.compact_header { 16.0 } else { 34.0 });
                    self.tab_button(
                        ui,
                        AppTab::Download,
                        "01",
                        "下载镜像",
                        layout.compact_header,
                    );
                    self.tab_button(ui, AppTab::Upgrade, "02", "升级固件", layout.compact_header);
                    self.tab_button(
                        ui,
                        AppTab::Advanced,
                        "03",
                        "高级功能",
                        layout.compact_header,
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // 只查一次工具路径，hover 提示与按钮文案共用结果。
                        let tool_path = self.tool_path();
                        let configured = tool_path.is_some();
                        let button_text = if configured {
                            "工具就绪"
                        } else {
                            "选择工具"
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!(
                                        "{}  {button_text}",
                                        if configured { "●" } else { "○" }
                                    ))
                                    .strong()
                                    .color(if configured {
                                        theme::SUCCESS
                                    } else {
                                        theme::WARNING
                                    }),
                                )
                                .fill(if configured {
                                    theme::ACCENT_SOFT
                                } else {
                                    theme::WARNING_SOFT
                                })
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    if configured {
                                        theme::ACCENT_DARK
                                    } else {
                                        theme::WARNING
                                    },
                                )),
                            )
                            .on_hover_text(
                                tool_path
                                    .map(|path| path.display().to_string())
                                    .unwrap_or_else(|| {
                                        format!("未找到 {}，点击手动选择", self.tool.name)
                                    }),
                            )
                            .clicked()
                        {
                            self.pick_tool();
                        }
                    });
                });
            });
    }

    fn tab_button(
        &mut self,
        ui: &mut egui::Ui,
        tab: AppTab,
        index: &str,
        label: &str,
        compact: bool,
    ) {
        let selected = self.tab == tab;
        let text = RichText::new(format!("{index}  {label}"))
            .size(13.0)
            .strong()
            .color(if selected { theme::TEXT } else { theme::MUTED });
        let response = ui
            .add_sized(
                [if compact { 102.0 } else { 112.0 }, 40.0],
                egui::Button::new(text)
                    .fill(if selected {
                        theme::PANEL_RAISED
                    } else {
                        theme::PANEL
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if selected {
                            theme::ACCENT_DARK
                        } else {
                            theme::BORDER_SOFT
                        },
                    )),
            )
            .on_hover_text(format!(
                "切换到{label} · Ctrl+{}",
                index.trim_start_matches('0')
            ));
        if selected {
            ui.painter().line_segment(
                [response.rect.left_bottom(), response.rect.right_bottom()],
                Stroke::new(2.0_f32, theme::ACCENT),
            );
        }
        if response.clicked() {
            self.tab = tab;
        }
    }

    pub(super) fn handle_shortcuts(&mut self, context: &egui::Context) {
        context.input(|input| {
            if input.modifiers.command && input.key_pressed(egui::Key::Num1) {
                self.tab = AppTab::Download;
            } else if input.modifiers.command && input.key_pressed(egui::Key::Num2) {
                self.tab = AppTab::Upgrade;
            } else if input.modifiers.command && input.key_pressed(egui::Key::Num3) {
                self.tab = AppTab::Advanced;
            }
        });

        if self.pending_action.is_some()
            && context.input(|input| input.key_pressed(egui::Key::Escape))
        {
            self.pending_action = None;
        }
    }

    pub(super) fn render_status(&mut self, context: &egui::Context) {
        egui::TopBottomPanel::bottom("device_status")
            .exact_height(42.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_DEEP)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER_SOFT))
                    .inner_margin(egui::Margin::symmetric(16, 5)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    let device_response = ui
                        .horizontal(|ui| {
                            let (dot_color, device_text) = self.device_label();
                            let (dot_rect, _) = ui
                                .allocate_exact_size(egui::vec2(12.0, 26.0), egui::Sense::hover());
                            ui.painter()
                                .circle_filled(dot_rect.center(), 4.0, dot_color);
                            ui.label(RichText::new(device_text).color(theme::TEXT));
                        })
                        .response;
                    self.render_device_hover_tip(device_response);

                    ui.separator();
                    ui.label(RichText::new(&self.status_text).color(theme::MUTED));

                    if self.is_task_running() {
                        ui.add_space(8.0);
                        ui.add(
                            egui::ProgressBar::new(self.progress)
                                .desired_width(220.0)
                                .show_percentage(),
                        );
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("v{APP_VERSION}"))
                                .color(theme::MUTED)
                                .small(),
                        );
                        if self.is_task_running()
                            && ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("停止任务").color(theme::DANGER),
                                    )
                                    .fill(theme::PANEL_RAISED),
                                )
                                .clicked()
                        {
                            self.cancel_task();
                        }
                    });
                });
            });
    }

    fn device_label(&self) -> (Color32, String) {
        match self.device_snapshot.devices.as_slice() {
            [] => {
                let suffix = self
                    .device_snapshot
                    .error
                    .as_deref()
                    .filter(|message| !message.is_empty())
                    .map(|_| "（扫描异常）")
                    .unwrap_or("");
                (theme::MUTED, format!("未发现设备{suffix}"))
            }
            [device] => (
                theme::SUCCESS,
                format!(
                    "{} · VID {} / PID {} · 位置 {}",
                    device.mode, device.vid, device.pid, device.location
                ),
            ),
            devices => (
                theme::WARNING,
                format!("发现 {} 台设备，请仅保留目标设备", devices.len()),
            ),
        }
    }

    pub(super) fn render_logs(&mut self, context: &egui::Context) {
        let operation_log = self.operation_log_display();
        let active = self.is_task_running();
        egui::TopBottomPanel::bottom("operation_logs")
            .resizable(self.logs_expanded)
            .default_height(if self.logs_expanded { 176.0 } else { 46.0 })
            .height_range(if self.logs_expanded {
                124.0..=320.0
            } else {
                46.0..=46.0
            })
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_DEEP)
                    .stroke(Stroke::new(1.0_f32, theme::BORDER_SOFT))
                    .inner_margin(egui::Margin::symmetric(18, 8)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("运行终端").strong().color(theme::TEXT));
                    status_badge(
                        ui,
                        if active { "记录中" } else { "已就绪" },
                        if active { theme::ACCENT } else { theme::MUTED },
                        if active {
                            theme::ACCENT_SOFT
                        } else {
                            theme::PANEL_RAISED
                        },
                    );
                    ui.label(
                        RichText::new(format!("{:04} EVENTS", self.logs.len()))
                            .monospace()
                            .small()
                            .color(theme::MUTED_DARK),
                    );
                    if let Some((file_name, full_path)) = &operation_log {
                        ui.separator();
                        ui.label(
                            RichText::new(file_name)
                                .monospace()
                                .size(11.0)
                                .color(theme::MUTED),
                        )
                        .on_hover_text(format!("磁盘日志：{full_path}"));
                    } else if !self.logs_expanded {
                        if let Some(entry) = self.logs.last() {
                            ui.separator();
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&entry.text)
                                        .monospace()
                                        .size(11.5)
                                        .color(theme::MUTED),
                                )
                                .truncate(),
                            );
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .small_button(if self.logs_expanded {
                                "收起"
                            } else {
                                "展开"
                            })
                            .clicked()
                        {
                            self.logs_expanded = !self.logs_expanded;
                        }
                        if ui
                            .small_button("清屏")
                            .on_hover_text("仅清除界面内容，不删除磁盘日志")
                            .clicked()
                        {
                            self.logs.clear();
                        }
                        if self.logs_expanded {
                            ui.checkbox(&mut self.follow_logs, "跟随输出");
                        }
                    });
                });
                if self.logs_expanded {
                    ui.separator();

                    // 逐行虚拟化渲染：只布局视口内的行，避免数千条日志拖慢每帧。
                    const LOG_ROW_HEIGHT: f32 = 21.0;
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(self.follow_logs)
                        .auto_shrink([false, false])
                        .show_rows(ui, LOG_ROW_HEIGHT, self.logs.len(), |ui, range| {
                            for entry in &self.logs[range] {
                                let (tag, color) = match entry.level {
                                    LogLevel::Info => ("INFO", theme::MUTED),
                                    LogLevel::Success => (" OK ", theme::SUCCESS),
                                    LogLevel::Warning => ("WARN", theme::WARNING),
                                    LogLevel::Error => ("ERR ", theme::DANGER),
                                    LogLevel::Command => ("RUN ", theme::ACCENT),
                                };
                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        [76.0, 18.0],
                                        egui::Label::new(
                                            RichText::new(entry.elapsed_label())
                                                .monospace()
                                                .size(10.5)
                                                .color(theme::MUTED_DARK),
                                        ),
                                    );
                                    ui.add_sized(
                                        [42.0, 18.0],
                                        egui::Label::new(
                                            RichText::new(tag).monospace().size(11.0).color(color),
                                        ),
                                    );
                                    ui.label(
                                        RichText::new(&entry.text)
                                            .monospace()
                                            .size(12.5)
                                            .color(theme::TEXT),
                                    );
                                });
                            }
                        });
                }
            });
    }

    pub(super) fn page_heading(
        &self,
        ui: &mut egui::Ui,
        index: &str,
        title: &str,
        description: &str,
    ) {
        let compact = LayoutProfile::from_width(ui.available_width()).compact_header;
        let heading = |ui: &mut egui::Ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format!("WORKSPACE / {index}"))
                        .monospace()
                        .size(10.0)
                        .color(theme::ACCENT),
                );
                ui.label(RichText::new(title).heading().strong().color(theme::TEXT));
                ui.label(RichText::new(description).color(theme::MUTED));
            });
        };
        let device_summary = |ui: &mut egui::Ui| {
            let (color, text) = self.device_label();
            let width = ui.available_width().min(290.0);
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(width, 34.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 3.0, theme::PANEL);
            ui.painter().rect_stroke(
                rect,
                3.0,
                Stroke::new(1.0_f32, theme::BORDER),
                egui::StrokeKind::Inside,
            );
            ui.painter()
                .circle_filled(egui::pos2(rect.left() + 14.0, rect.center().y), 3.5, color);
            ui.painter().text(
                egui::pos2(rect.left() + 26.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::proportional(12.0),
                theme::MUTED,
            );
            self.render_device_hover_tip(response);
        };

        if compact {
            heading(ui);
            ui.add_space(10.0);
            device_summary(ui);
        } else {
            ui.horizontal(|ui| {
                heading(ui);
                ui.with_layout(Layout::right_to_left(Align::Center), device_summary);
            });
        }
        ui.add_space(16.0);
    }
}
