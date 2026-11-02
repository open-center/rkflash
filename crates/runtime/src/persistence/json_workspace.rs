use rkflash_application::WorkspaceRepository;
use rkflash_config::{decode_workspace, encode_workspace, WorkspaceConfig};
use std::fs;
use std::path::Path;

pub struct JsonConfigurationRepository;

impl WorkspaceRepository for JsonConfigurationRepository {
    fn load_workspace(&self, path: &Path) -> Result<WorkspaceConfig, String> {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("读取配置失败（{}）：{error}", path.display()))?;
        decode_workspace(&text)
            .map_err(|error| format!("解析配置失败（{}）：{error}", path.display()))
    }

    fn save_workspace(&self, path: &Path, config: &WorkspaceConfig) -> Result<(), String> {
        let text = encode_workspace(config).map_err(|error| format!("序列化配置失败：{error}"))?;
        // 先写同目录临时文件再原子替换：写一半时崩溃或断电不会损坏原有配置。
        let temporary = path.with_file_name(format!(
            ".{}.{}.tmp",
            std::process::id(),
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "workspace".to_owned())
        ));
        if let Err(error) = fs::write(&temporary, &text) {
            return Err(format!("写入配置失败（{}）：{error}", temporary.display()));
        }
        fs::rename(&temporary, path).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            format!("写入配置失败（{}）：{error}", path.display())
        })
    }
}
