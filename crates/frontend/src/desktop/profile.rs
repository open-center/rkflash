//! 平台向界面注入的差异；具体取值由 `platforms/*` 提供，界面代码本身不含平台条件编译。

pub use eframe::Renderer;

pub struct DesktopProfile {
    /// 中文字体候选（路径, TTC 表面索引），按顺序取第一个存在的文件。
    pub cjk_fonts: &'static [(&'static str, u32)],
    /// 依次尝试的渲染器（渲染器, 显示名称）；前一个创建窗口失败时换下一个。
    pub renderers: &'static [(Renderer, &'static str)],
    /// 图形界面启动失败时的提示；发布版没有控制台的平台用对话框告知原因。
    pub show_startup_error: fn(&str),
}

/// 以系统对话框显示错误，供没有控制台的平台作为 `show_startup_error` 使用。
pub fn show_error_dialog(message: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("RKFlash")
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}
