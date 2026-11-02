//! 与界面和基础设施无关的用例、端口与应用门面。

mod facade;
mod image_resolution;
mod ports;
mod validation;

pub(crate) use facade::ApplicationPorts;
pub use facade::FlashApplication;

pub(crate) use ports::{
    DeviceMonitor, FileSystem, FlashPlanner, FlashProfile, OperationExecutor, OperationLog,
    ToolLocator, ToolProtocol, WorkspaceRepository,
};
