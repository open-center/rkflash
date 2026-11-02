//! Rockchip profile、协议解析与命令规划适配器。
//!
//! Linux 使用 Rockchip 官方 `upgrade_tool`；Windows 使用开源 `rkdeveloptool`，
//! 其固件解析与整包升级工作流来自平台无关的 rkflash-core。

mod discovery;
mod parameter;
mod parser;
mod profile;
mod rkdeveloptool;
mod upgrade_tool;

pub(crate) use discovery::{discover_rkdeveloptool, discover_upgrade_tool};
pub(crate) use rkdeveloptool::RkDevelopToolAdapter;
pub(crate) use upgrade_tool::UpgradeToolAdapter;
