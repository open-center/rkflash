//! RKFlash 应用层：用例门面、操作校验、执行计划与端口定义。
//!
//! 只依赖契约（rkflash-api）、配置模型与领域核心（rkflash-core）；进程、文件系统和
//! 烧录工具都通过 [`ports`] 中的接口注入，由通用运行时与各平台层实现。

pub mod domain;
mod facade;
mod image_resolution;
pub mod ports;
mod profile;
mod validation;

pub use facade::{ApplicationPorts, FlashApplication};
pub use ports::{
    DeviceMonitor, FileSystem, FlashPlanner, FlashProfile, OperationExecutor, OperationLog,
    ToolLocator, ToolProtocol, WorkspaceRepository,
};
pub use profile::RockchipImageProfile;
