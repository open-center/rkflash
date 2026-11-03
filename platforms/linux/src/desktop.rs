//! Linux 桌面界面的平台差异：中文字体、渲染器与启动失败提示。

use rkflash_frontend::desktop::{DesktopProfile, Renderer};

/// Noto CJK .ttc 内置 JP/KR/SC/TC/HK 多个表面，简体中文字形位于索引 2；
/// 其余集合字体取第一个表面。末尾的 PingFang 供在 macOS 上开发预览时使用。
const CJK_FONTS: &[(&str, u32)] = &[
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 2),
    ("/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc", 2),
    ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 2),
    (
        "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
        0,
    ),
    ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0),
    ("/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc", 0),
    (
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        0,
    ),
    ("/usr/share/fonts/truetype/arphic/uming.ttc", 0),
    ("/System/Library/Fonts/PingFang.ttc", 0),
];

pub fn desktop_profile() -> DesktopProfile {
    DesktopProfile {
        cjk_fonts: CJK_FONTS,
        renderers: &[(Renderer::Glow, "OpenGL")],
        // 从终端或桌面文件启动，失败原因由启动器输出到标准错误。
        show_startup_error: |_| {},
    }
}
