//! 系统中文字体发现；不承载工作区、固件或设备文件访问。

use std::path::Path;

pub(in crate::desktop) struct CjkFont {
    pub data: Vec<u8>,
    /// TTC 集合文件中的字形表面索引；普通字体为 0。
    pub face_index: u32,
}

pub(in crate::desktop) fn load_cjk_font() -> Option<CjkFont> {
    // (路径, TTC 字体索引)。Noto CJK .ttc 内置 JP/KR/SC/TC/HK 多个表面，
    // 简体中文字形位于索引 2；其余集合字体默认取第一个表面。
    let candidates: &[(&str, u32)] = &[
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
        ("C:\\Windows\\Fonts\\msyh.ttc", 0),
        ("C:\\Windows\\Fonts\\simhei.ttf", 0),
        ("/System/Library/Fonts/PingFang.ttc", 0),
    ];

    candidates
        .iter()
        .find(|(path, _)| Path::new(path).is_file())
        .and_then(|(path, face_index)| {
            std::fs::read(path).ok().map(|data| CjkFont {
                data,
                face_index: *face_index,
            })
        })
}
