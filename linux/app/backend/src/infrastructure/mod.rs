//! 后端内部的文件、进程与 Rockchip 适配器实现。

mod persistence;
mod process;
mod rockchip;

pub(crate) use persistence::{JsonConfigurationRepository, LocalFileSystem};
pub(crate) use process::ProcessRuntime;
pub(crate) use rockchip::{discover_upgrade_tool, UpgradeToolAdapter};
