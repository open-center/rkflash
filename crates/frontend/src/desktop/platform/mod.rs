//! 桌面平台适配，只处理界面自身需要的操作系统资源。

mod font;
mod window;

pub(in crate::desktop) use font::load_cjk_font;
pub(in crate::desktop) use window::{show_startup_error, RENDERERS};
