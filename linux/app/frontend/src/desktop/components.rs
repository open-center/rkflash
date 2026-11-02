//! 桌面页面复用的视觉组件。

use super::theme;
use eframe::egui::{self, Align, Color32, Layout, RichText, Stroke};

pub(super) fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0_f32, theme::BORDER))
        .inner_margin(egui::Margin::same(18))
}

pub(super) fn inset() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::CANVAS)
        .stroke(Stroke::new(1.0_f32, theme::BORDER_SOFT))
        .inner_margin(egui::Margin::same(12))
}

pub(super) fn status_badge(ui: &mut egui::Ui, label: &str, accent: Color32, fill: Color32) {
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, accent.gamma_multiply(0.55)))
        .corner_radius(3.0)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let (dot, _) = ui.allocate_exact_size(egui::vec2(7.0, 12.0), egui::Sense::hover());
                ui.painter().circle_filled(dot.center(), 3.0, accent);
                ui.label(RichText::new(label).strong().size(11.0).color(accent));
            });
        });
}

pub(super) fn primary_button(label: &str, fill: Color32) -> egui::Button<'_> {
    let stroke = if fill == theme::ACCENT_DARK {
        theme::ACCENT
    } else {
        theme::DANGER
    };
    egui::Button::new(RichText::new(label).strong().color(Color32::WHITE))
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, stroke))
        .min_size(egui::vec2(128.0, 40.0))
}

pub(super) fn danger_button(label: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(label).strong().color(theme::DANGER))
        .fill(theme::DANGER_SOFT)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(122, 52, 59)))
        .min_size(egui::vec2(112.0, 38.0))
}

pub(super) fn section_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.horizontal(|ui| {
        let (marker, _) = ui.allocate_exact_size(egui::vec2(3.0, 34.0), egui::Sense::hover());
        ui.painter().rect_filled(marker, 1.5, theme::ACCENT_DARK);
        ui.vertical(|ui| {
            ui.label(RichText::new(title).strong().size(16.0).color(theme::TEXT));
            ui.label(RichText::new(subtitle).small().color(theme::MUTED));
        });
    });
    ui.add_space(12.0);
}

pub(super) fn button_grid(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(7.0, 7.0);
        contents(ui);
    });
}

pub(super) fn metric(ui: &mut egui::Ui, label: &str, value: String) {
    ui.label(
        RichText::new(label.to_uppercase())
            .size(10.0)
            .color(theme::MUTED_DARK),
    );
    ui.label(RichText::new(value).strong().size(15.0).color(theme::TEXT));
}

pub(super) fn stat_strip(ui: &mut egui::Ui, items: &[(&str, String)]) {
    inset().show(ui, |ui| {
        ui.horizontal(|ui| {
            for (index, (label, value)) in items.iter().enumerate() {
                if index > 0 {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                }
                ui.vertical(|ui| metric(ui, label, value.clone()));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new("RK DEVICE PIPELINE")
                        .monospace()
                        .size(10.0)
                        .color(theme::MUTED_DARK),
                );
            });
        });
    });
}

pub(super) fn field(ui: &mut egui::Ui, label: &str, value: &mut String, width: f32) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).small().color(theme::MUTED));
        ui.add_sized(
            [width, 30.0],
            egui::TextEdit::singleline(value).font(egui::TextStyle::Monospace),
        );
    });
}

/// 路径输入框；返回是否请求打开文件对话框，由调用方在后台线程执行。
pub(super) fn path_picker(ui: &mut egui::Ui, value: &mut String, hint: &str) -> bool {
    let mut requested = false;
    ui.horizontal(|ui| {
        ui.add_sized(
            [ui.available_width() - 74.0, 34.0],
            egui::TextEdit::singleline(value).hint_text(hint),
        );
        if ui.button("选择").clicked() {
            requested = true;
        }
    });
    requested
}

pub(super) fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    match bytes as f64 {
        value if value >= GIB => format!("{:.2} GiB", value / GIB),
        value if value >= MIB => format!("{:.2} MiB", value / MIB),
        value if value >= KIB => format!("{:.2} KiB", value / KIB),
        _ => format!("{bytes} B"),
    }
}
