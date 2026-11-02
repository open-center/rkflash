//! 桌面视觉令牌与中文字体装配。

use super::platform::load_cjk_font;
use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, TextStyle};

pub const CANVAS: Color32 = Color32::from_rgb(9, 14, 18);
pub const SURFACE_DEEP: Color32 = Color32::from_rgb(7, 11, 14);
pub const PANEL: Color32 = Color32::from_rgb(15, 22, 27);
pub const PANEL_RAISED: Color32 = Color32::from_rgb(21, 30, 36);
pub const PANEL_HOVER: Color32 = Color32::from_rgb(27, 39, 45);
pub const BORDER: Color32 = Color32::from_rgb(43, 57, 64);
pub const BORDER_SOFT: Color32 = Color32::from_rgb(29, 41, 47);
pub const TEXT: Color32 = Color32::from_rgb(232, 239, 239);
pub const MUTED: Color32 = Color32::from_rgb(137, 155, 159);
pub const MUTED_DARK: Color32 = Color32::from_rgb(87, 105, 111);
pub const ACCENT: Color32 = Color32::from_rgb(55, 215, 184);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(18, 128, 108);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(18, 54, 50);
pub const WARNING: Color32 = Color32::from_rgb(242, 176, 82);
pub const WARNING_SOFT: Color32 = Color32::from_rgb(59, 43, 24);
pub const DANGER: Color32 = Color32::from_rgb(244, 102, 111);
pub const DANGER_SOFT: Color32 = Color32::from_rgb(62, 31, 36);
pub const SUCCESS: Color32 = Color32::from_rgb(89, 214, 143);

/// 应用视觉样式并装配中文字体；返回中文字体是否可用，调用方据此提示用户。
pub fn install(context: &egui::Context) -> bool {
    let mut style = (*context.style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = CANVAS;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = SURFACE_DEEP;
    style.visuals.faint_bg_color = PANEL_RAISED;
    style.visuals.code_bg_color = SURFACE_DEEP;
    style.visuals.selection.bg_fill = ACCENT_DARK;
    style.visuals.selection.stroke.color = ACCENT;
    style.visuals.widgets.noninteractive.bg_fill = PANEL;
    style.visuals.widgets.noninteractive.bg_stroke.color = BORDER;
    style.visuals.widgets.noninteractive.fg_stroke.color = TEXT;
    style.visuals.widgets.inactive.bg_fill = PANEL_RAISED;
    style.visuals.widgets.inactive.bg_stroke.color = BORDER;
    style.visuals.widgets.inactive.fg_stroke.color = TEXT;
    style.visuals.widgets.hovered.bg_fill = PANEL_HOVER;
    style.visuals.widgets.hovered.bg_stroke.color = ACCENT_DARK;
    style.visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
    style.visuals.widgets.active.bg_fill = ACCENT_DARK;
    style.visuals.widgets.active.bg_stroke.color = ACCENT;
    style.spacing.item_spacing = egui::vec2(10.0, 9.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.interact_size.y = 36.0;
    style.text_styles = [
        (TextStyle::Heading, FontId::proportional(24.0)),
        (TextStyle::Body, FontId::proportional(14.0)),
        (TextStyle::Monospace, FontId::monospace(13.0)),
        (TextStyle::Button, FontId::proportional(14.0)),
        (TextStyle::Small, FontId::proportional(12.0)),
    ]
    .into();
    context.set_style(style);
    install_cjk_font(context)
}

fn install_cjk_font(context: &egui::Context) -> bool {
    let Some(font) = load_cjk_font() else {
        // 静默降级会让中文全部渲染成豆腐块；返回状态由调用方提示用户。
        return false;
    };

    let mut fonts = FontDefinitions::default();
    let mut data = FontData::from_owned(font.data);
    data.index = font.face_index;
    fonts.font_data.insert("cjk".to_owned(), data.into());
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, "cjk".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.push("cjk".to_owned());
    }
    context.set_fonts(fonts);
    true
}
