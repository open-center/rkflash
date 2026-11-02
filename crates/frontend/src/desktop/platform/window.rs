//! 窗口渲染与启动失败提示的平台差异。

/// Windows 优先使用 wgpu（Direct3D 12，无显卡时由系统 WARP 软件渲染），失败再试 OpenGL；
/// 其他平台沿用已验证的 OpenGL 渲染。
#[cfg(windows)]
pub(in crate::desktop) const RENDERERS: &[(eframe::Renderer, &str)] = &[
    (eframe::Renderer::Wgpu, "Direct3D/wgpu"),
    (eframe::Renderer::Glow, "OpenGL"),
];
#[cfg(not(windows))]
pub(in crate::desktop) const RENDERERS: &[(eframe::Renderer, &str)] =
    &[(eframe::Renderer::Glow, "OpenGL")];

/// Windows 发布版没有控制台，启动失败时用对话框告知原因。
#[cfg(windows)]
pub(in crate::desktop) fn show_startup_error(message: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("RKFlash")
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

#[cfg(not(windows))]
pub(in crate::desktop) fn show_startup_error(_message: &str) {}
