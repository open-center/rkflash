//! 系统中文字体加载；候选路径由平台的 `DesktopProfile` 提供。

use std::path::Path;

pub(in crate::desktop) struct CjkFont {
    pub data: Vec<u8>,
    /// TTC 集合文件中的字形表面索引；普通字体为 0。
    pub face_index: u32,
}

/// 按顺序取第一个存在的候选字体（路径, TTC 表面索引）。
pub(in crate::desktop) fn load_cjk_font(candidates: &[(&str, u32)]) -> Option<CjkFont> {
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
