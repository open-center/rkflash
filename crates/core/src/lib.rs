//! RKFlash 平台无关核心：Rockchip 固件格式、分区表、芯片识别与基于 rkdeveloptool 的烧录工作流。
//!
//! 本 crate 不依赖界面、配置模型或进程管理：外部工具只通过 [`session::ToolSession`] 调用，
//! 因此 Linux、Windows 的 Rust 应用与（后续）macOS 版都可以复用同一套规则。

pub mod chip;
pub mod firmware;
mod md5;
pub mod output;
pub mod partition;
pub mod rkdeveloptool;
pub mod session;
pub mod workflow;
