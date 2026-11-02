//! parameter 分区与同目录镜像的纯匹配规则。

use rkflash_config::ImageEntry;
use std::path::{Path, PathBuf};

pub(super) fn resolve_image_paths(
    entries: &mut [ImageEntry],
    parameter_path: &Path,
    files: &[PathBuf],
) {
    for entry in entries {
        let partition_name = entry
            .name
            .split_once(':')
            .map(|(name, _)| name)
            .unwrap_or(&entry.name);

        let matched = if partition_name.eq_ignore_ascii_case("parameter") {
            Some(parameter_path)
        } else {
            ["img", "bin", "txt"]
                .into_iter()
                .find_map(|extension| {
                    files.iter().find_map(|file| {
                        let stem = file.file_stem()?.to_str()?;
                        let file_extension = file.extension()?.to_str()?;
                        (stem.eq_ignore_ascii_case(partition_name)
                            && file_extension.eq_ignore_ascii_case(extension))
                        .then_some(file.as_path())
                    })
                })
                .or_else(|| {
                    files.iter().find_map(|file| {
                        let file_name = file.file_name()?.to_str()?;
                        file_name
                            .eq_ignore_ascii_case(partition_name)
                            .then_some(file.as_path())
                    })
                })
        };

        if let Some(file) = matched {
            entry.path = file.display().to_string();
            entry.enabled = true;
        } else {
            entry.path.clear();
            entry.enabled = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_image_paths;
    use rkflash_config::ImageEntry;
    use std::path::{Path, PathBuf};

    #[test]
    fn resolves_parameter_and_partition_images_from_same_directory() {
        let parameter = Path::new("/firmware/parameter.txt");
        let files = vec![
            PathBuf::from("/firmware/parameter.txt"),
            PathBuf::from("/firmware/UBOOT.img"),
            PathBuf::from("/firmware/rootfs.img"),
        ];
        let mut entries = vec![
            ImageEntry::new(false, "0x0", "parameter"),
            ImageEntry::new(false, "0x4000", "uboot"),
            ImageEntry::new(false, "0x8000", "rootfs:grow"),
            ImageEntry::new(true, "0xa000", "vendor"),
        ];

        resolve_image_paths(&mut entries, parameter, &files);

        assert_eq!(entries[0].path, "/firmware/parameter.txt");
        assert_eq!(entries[1].path, "/firmware/UBOOT.img");
        assert_eq!(entries[2].path, "/firmware/rootfs.img");
        assert!(entries[..3].iter().all(|entry| entry.enabled));
        assert_eq!(entries[3].path, "");
        assert!(!entries[3].enabled);
    }
}
