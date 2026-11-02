//! RKTools 可序列化配置模型与 JSON 编解码。

mod codec;
mod model;

pub use codec::{decode_workspace, encode_workspace};
pub use model::{ImageEntry, WorkspaceConfig, WORKSPACE_FORMAT_VERSION};
