use crate::WorkspaceConfig;

pub fn decode_workspace(text: &str) -> Result<WorkspaceConfig, serde_json::Error> {
    serde_json::from_str(text)
}

pub fn encode_workspace(config: &WorkspaceConfig) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(config)
}

#[cfg(test)]
mod tests {
    use super::{decode_workspace, encode_workspace};
    use crate::{ImageEntry, WorkspaceConfig, WORKSPACE_FORMAT_VERSION};

    #[test]
    fn round_trips_workspace_configuration() {
        let config = WorkspaceConfig {
            format_version: WORKSPACE_FORMAT_VERSION,
            images: vec![ImageEntry::new(true, "0x00008000", "boot")],
            firmware_path: "update.img".to_owned(),
            loader_path: "MiniLoaderAll.bin".to_owned(),
            reset_after_download: true,
            no_reset_after_upgrade: false,
        };

        let encoded = encode_workspace(&config).expect("encode workspace");
        assert_eq!(
            decode_workspace(&encoded).expect("decode workspace"),
            config
        );
    }
}
