//! Windows 桌面界面的平台差异：中文字体、渲染器与启动失败提示。

use rkflash_frontend::desktop::{show_error_dialog, DesktopProfile, Renderer};

const CJK_FONTS: &[(&str, u32)] = &[
    ("C:\\Windows\\Fonts\\msyh.ttc", 0),
    ("C:\\Windows\\Fonts\\simhei.ttf", 0),
];

pub fn desktop_profile() -> DesktopProfile {
    DesktopProfile {
        cjk_fonts: CJK_FONTS,
        // 优先 wgpu（Direct3D 12，无显卡时由系统 WARP 软件渲染），失败再试 OpenGL。
        renderers: &[
            (Renderer::Wgpu, "Direct3D/wgpu"),
            (Renderer::Glow, "OpenGL"),
        ],
        // 发布版没有控制台，启动失败时用对话框告知原因。
        show_startup_error: show_error_dialog,
    }
}
