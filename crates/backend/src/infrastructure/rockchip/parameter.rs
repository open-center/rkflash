//! 把 parameter 分区映射为下载列表条目；解析规则位于 rkflash-core。

use rkflash_config::ImageEntry;
use rkflash_core::partition::parse_parameter_partitions;

pub(super) fn parse_parameter(text: &str) -> Result<Vec<ImageEntry>, String> {
    Ok(parse_parameter_partitions(text)?
        .into_iter()
        .map(|partition| ImageEntry::new(false, &partition.address, &partition.name))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::parse_parameter;

    #[test]
    fn parses_rockchip_parameter_layout() {
        let text =
            "CMDLINE:mtdparts=rk29xxnand:0x00002000@0x00002000(uboot),-@0x00004000(rootfs:grow)";
        let result = parse_parameter(text).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "uboot");
        assert_eq!(result[0].address, "0x00002000");
        assert_eq!(result[1].name, "rootfs:grow");
    }
}
