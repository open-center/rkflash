//! 在生成后端命令前执行与工具无关的输入校验。

use super::FileSystem;
use crate::domain::{normalize_sector, FlashOperation};
use std::path::Path;

pub(super) fn validate_operation(
    operation: &FlashOperation,
    file_system: &dyn FileSystem,
) -> Result<(), String> {
    match operation {
        FlashOperation::DownloadImages { images, .. } => {
            let selected: Vec<_> = images.iter().filter(|image| image.enabled).collect();
            if selected.is_empty() {
                return Err("没有勾选任何下载项".to_owned());
            }
            for image in selected {
                require_file(file_system, &image.path, &format!("下载项“{}”", image.name))?;
                if !image.address.trim().is_empty() {
                    validate_sector(&image.address, &format!("下载项“{}”起始扇区", image.name))?;
                }
            }
        }
        FlashOperation::UpgradeFirmware { firmware, .. }
        | FlashOperation::EraseFlash { firmware } => {
            require_path_file(file_system, firmware, "固件")?;
        }
        FlashOperation::UpgradeLoader { loader, .. } => {
            require_path_file(file_system, loader, "Loader")?;
        }
        FlashOperation::ReadLba {
            start,
            count,
            output,
        } => {
            validate_sector(start, "起始扇区")?;
            validate_sector(count, "扇区数量")?;
            require_output(file_system, output, "LBA 输出文件")?;
        }
        FlashOperation::WriteLba { start, image } => {
            validate_sector(start, "起始扇区")?;
            require_path_file(file_system, image, "LBA 镜像")?;
        }
        FlashOperation::EraseLba { start, count } => {
            validate_sector(start, "起始扇区")?;
            validate_sector(count, "扇区数量")?;
        }
        FlashOperation::ExportSerialLog { output } => {
            require_output(file_system, output, "串口日志输出文件")?;
        }
        FlashOperation::Query { .. } | FlashOperation::Control(_) => {}
    }
    Ok(())
}

fn require_file(file_system: &dyn FileSystem, path: &str, label: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err(format!("{label}尚未选择文件"));
    }
    require_path_file(file_system, Path::new(path), label)
}

fn require_path_file(file_system: &dyn FileSystem, path: &Path, label: &str) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err(format!("{label}尚未选择文件"));
    }
    if !file_system.is_file(path) {
        return Err(format!("{label}文件不存在：{}", path.display()));
    }
    Ok(())
}

fn require_output(file_system: &dyn FileSystem, path: &Path, label: &str) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err(format!("{label}路径为空"));
    }
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        if !file_system.is_directory(parent) {
            return Err(format!("{label}所在目录不存在：{}", parent.display()));
        }
    }
    Ok(())
}

fn validate_sector(value: &str, label: &str) -> Result<(), String> {
    normalize_sector(value).map(|_| ()).map_err(|_| {
        format!(
            "{label}必须是十进制数或 0x 开头的十六进制数：{}",
            value.trim()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::validate_sector;

    #[test]
    fn accepts_decimal_and_hex_sectors() {
        assert!(validate_sector("8192", "地址").is_ok());
        assert!(validate_sector("0x00002000", "地址").is_ok());
    }

    #[test]
    fn rejects_invalid_sector() {
        assert!(validate_sector("0xGG", "地址").is_err());
    }
}
