//! 应用层只依赖这些端口，不直接访问进程、文件系统或厂商实现。

mod execution;
mod persistence;
mod planning;

pub use execution::{DeviceMonitor, OperationExecutor, OperationLog, ToolLocator};
pub use persistence::{FileSystem, WorkspaceRepository};
pub use planning::{FlashPlanner, FlashProfile, ToolProtocol};
