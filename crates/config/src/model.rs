use serde::{Deserialize, Serialize};

pub const WORKSPACE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageEntry {
    pub enabled: bool,
    pub address: String,
    pub name: String,
    pub path: String,
}

impl ImageEntry {
    pub fn new(enabled: bool, address: &str, name: &str) -> Self {
        Self {
            enabled,
            address: address.to_owned(),
            name: name.to_owned(),
            path: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub format_version: u32,
    pub images: Vec<ImageEntry>,
    pub firmware_path: String,
    pub loader_path: String,
    pub reset_after_download: bool,
    pub no_reset_after_upgrade: bool,
}
