//! 错误、设备状态与高风险操作浮层。

use super::*;

impl RkFlashApp {
    pub(super) fn render_error_toast(&mut self, context: &egui::Context) {
        if self
            .error_toast
            .as_ref()
            .is_some_and(ErrorToast::is_expired)
        {
            self.error_toast = None;
            return;
        }
        let Some(toast) = self.error_toast.as_ref() else {
            return;
        };

        let mut close = false;
        let mut view_logs = false;
        let operation_log = self.operation_log_display();
        egui::Area::new(egui::Id::new("operation_error_toast"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-18.0, 90.0))
            .order(egui::Order::Foreground)
            .show(context, |ui| {
                egui::Frame::new()
                    .fill(theme::PANEL_RAISED)
                    .stroke(Stroke::new(1.0_f32, theme::DANGER))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 8],
                        blur: 18,
                        spread: 0,
                        color: Color32::from_black_alpha(160),
                    })
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(400.0);
                        ui.horizontal(|ui| {
                            let (icon_rect, _) = ui
                                .allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                            ui.painter().circle_filled(
                                icon_rect.center(),
                                13.0,
                                Color32::from_rgb(75, 30, 35),
                            );
                            ui.painter().text(
                                icon_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "×",
                                egui::FontId::proportional(17.0),
                                theme::DANGER,
                            );
                            ui.label(
                                RichText::new(&toast.title)
                                    .strong()
                                    .size(15.0)
                                    .color(theme::DANGER),
                            );
                            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("×").size(17.0).color(theme::MUTED),
                                        )
                                        .frame(false),
                                    )
                                    .on_hover_text("关闭提示")
                                    .clicked()
                                {
                                    close = true;
                                }
                            });
                        });
                        ui.add_space(5.0);
                        ui.label(RichText::new(&toast.detail).color(theme::TEXT));
                        if let Some((file_name, full_path)) = &operation_log {
                            ui.add_space(7.0);
                            inset().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("已保留").size(11.0).color(theme::MUTED_DARK),
                                    );
                                    ui.label(
                                        RichText::new(file_name)
                                            .monospace()
                                            .size(11.5)
                                            .color(theme::MUTED),
                                    )
                                    .on_hover_text(full_path);
                                });
                            });
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("展开运行终端")
                                                .strong()
                                                .color(theme::DANGER),
                                        )
                                        .fill(Color32::from_rgb(75, 30, 35))
                                        .stroke(Stroke::new(1.0_f32, theme::DANGER)),
                                    )
                                    .clicked()
                                {
                                    view_logs = true;
                                }
                            });
                        });
                        ui.add_space(5.0);
                        let (bar, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 2.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(bar, 1.0, theme::BORDER_SOFT);
                        let remaining = egui::Rect::from_min_size(
                            bar.min,
                            egui::vec2(bar.width() * toast.remaining_fraction(), bar.height()),
                        );
                        ui.painter().rect_filled(remaining, 1.0, theme::DANGER);
                    });
            });

        if view_logs {
            self.logs_expanded = true;
            self.follow_logs = true;
            self.error_toast = None;
        } else if close {
            self.error_toast = None;
        } else {
            context.request_repaint_after(Duration::from_millis(100));
        }
    }

    pub(super) fn render_operation_tip(&mut self, context: &egui::Context) {
        if self
            .operation_tip
            .as_ref()
            .is_some_and(OperationTip::is_expired)
        {
            self.operation_tip = None;
            return;
        }
        let Some(tip) = self.operation_tip.as_ref() else {
            return;
        };

        let guidance = tip.mode.guidance();
        let (accent, soft_fill, icon) = match tip.mode {
            DeviceMode::Loader => (theme::SUCCESS, theme::ACCENT_SOFT, "✓"),
            DeviceMode::Maskrom => (theme::WARNING, theme::WARNING_SOFT, "!"),
        };
        let mut close = false;

        let vertical_offset = if self.error_toast.is_some() {
            285.0
        } else {
            90.0
        };
        egui::Area::new(egui::Id::new("device_operation_tip"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-18.0, vertical_offset))
            .order(egui::Order::Foreground)
            .show(context, |ui| {
                egui::Frame::new()
                    .fill(theme::PANEL_RAISED)
                    .stroke(Stroke::new(1.0_f32, accent))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 8],
                        blur: 18,
                        spread: 0,
                        color: Color32::from_black_alpha(150),
                    })
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width(360.0);
                        ui.horizontal(|ui| {
                            let (icon_rect, _) = ui
                                .allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                            ui.painter()
                                .circle_filled(icon_rect.center(), 13.0, soft_fill);
                            ui.painter().text(
                                icon_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                icon,
                                egui::FontId::proportional(17.0),
                                accent,
                            );
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(guidance.title)
                                        .strong()
                                        .size(15.0)
                                        .color(theme::TEXT),
                                );
                                ui.label(RichText::new(guidance.detail).color(theme::MUTED));
                            });
                            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("×").size(17.0).color(theme::MUTED),
                                        )
                                        .frame(false),
                                    )
                                    .on_hover_text("关闭提示")
                                    .clicked()
                                {
                                    close = true;
                                }
                            });
                        });
                        ui.add_space(5.0);
                        let (bar, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 2.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(bar, 1.0, theme::BORDER_SOFT);
                        let remaining = egui::Rect::from_min_size(
                            bar.min,
                            egui::vec2(bar.width() * tip.remaining_fraction(), bar.height()),
                        );
                        ui.painter().rect_filled(remaining, 1.0, accent);
                    });
            });

        if close {
            self.operation_tip = None;
        } else {
            context.request_repaint_after(Duration::from_millis(100));
        }
    }

    pub(super) fn render_confirmation(&mut self, context: &egui::Context) {
        let Some(action) = self.pending_action.clone() else {
            return;
        };
        let mut execute = false;
        let mut cancel = false;
        egui::Window::new(&action.title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .fixed_size(egui::vec2(450.0, 190.0))
            .show(context, |ui| {
                ui.label(
                    RichText::new(if action.danger {
                        "高风险操作"
                    } else {
                        "设备状态变更"
                    })
                    .strong()
                    .color(if action.danger {
                        theme::DANGER
                    } else {
                        theme::WARNING
                    }),
                );
                ui.add_space(8.0);
                ui.label(RichText::new(&action.detail).color(theme::TEXT));
                ui.add_space(16.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(primary_button(
                            "确认执行",
                            if action.danger {
                                Color32::from_rgb(145, 48, 55)
                            } else {
                                theme::ACCENT_DARK
                            },
                        ))
                        .clicked()
                    {
                        execute = true;
                    }
                    if ui.button("取消").clicked() {
                        cancel = true;
                    }
                });
            });

        if execute {
            self.pending_action = None;
            self.run_confirmed(action.action);
        } else if cancel {
            self.pending_action = None;
        }
    }
}
