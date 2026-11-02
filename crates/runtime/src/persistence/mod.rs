//! 工作区与文本输入的文件系统适配器。

mod json_workspace;
mod local_file_system;

pub use json_workspace::JsonConfigurationRepository;
pub use local_file_system::LocalFileSystem;
