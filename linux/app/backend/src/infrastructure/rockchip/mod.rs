//! Rockchip profile、协议解析与命令规划适配器。

mod discovery;
mod parameter;
mod parser;
mod profile;
mod upgrade_tool;

pub(crate) use discovery::discover_upgrade_tool;
pub(crate) use upgrade_tool::UpgradeToolAdapter;
