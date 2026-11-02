//! Rockchip profile、协议解析与命令规划适配器。
//!
//! Linux 使用 Rockchip 官方 `upgrade_tool`；Windows 使用开源 `rkdeveloptool`
//! （内置 libusb），并由本模块的固件解析器实现 update.img 整包升级。

mod discovery;
mod firmware;
mod md5;
mod parameter;
mod parser;
mod profile;
mod rkdeveloptool;
mod upgrade_tool;
mod workflow;

pub(crate) use discovery::{discover_rkdeveloptool, discover_upgrade_tool};
pub(crate) use rkdeveloptool::RkDevelopToolAdapter;
pub(crate) use upgrade_tool::UpgradeToolAdapter;
