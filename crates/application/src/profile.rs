//! 两个平台共用的 Rockchip 下载列表规则：默认分区与 parameter 导入，均来自 rkflash-core。

use crate::ports::FlashProfile;
use rkflash_config::ImageEntry;
use rkflash_core::partition::{parse_parameter_partitions, STANDARD_PARTITIONS};

#[derive(Default)]
pub struct RockchipImageProfile;

impl FlashProfile for RockchipImageProfile {
    fn default_images(&self) -> Vec<ImageEntry> {
        STANDARD_PARTITIONS
            .iter()
            .map(|(name, address)| ImageEntry::new(false, address, name))
            .collect()
    }

    fn parse_parameter(&self, text: &str) -> Result<Vec<ImageEntry>, String> {
        Ok(parse_parameter_partitions(text)?
            .into_iter()
            .map(|partition| ImageEntry::new(false, &partition.address, &partition.name))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::RockchipImageProfile;
    use crate::ports::FlashProfile;

    #[test]
    fn default_list_contains_disabled_standard_partitions() {
        let images = RockchipImageProfile.default_images();
        assert_eq!(
            images.first().map(|image| image.name.as_str()),
            Some("parameter")
        );
        assert!(images.iter().all(|image| !image.enabled));
    }

    #[test]
    fn imports_parameter_partitions_as_disabled_entries() {
        let text =
            "CMDLINE:mtdparts=rk29xxnand:0x00002000@0x00002000(uboot),-@0x00004000(rootfs:grow)";
        let images = RockchipImageProfile.parse_parameter(text).unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].address, "0x00002000");
        assert_eq!(images[1].name, "rootfs:grow");
    }
}
