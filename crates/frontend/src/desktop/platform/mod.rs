//! 桌面平台适配，只处理界面自身需要的操作系统资源。

mod font;

pub(in crate::desktop) use font::load_cjk_font;
