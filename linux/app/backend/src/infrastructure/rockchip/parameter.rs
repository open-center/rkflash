use rkflash_config::ImageEntry;

pub(super) fn parse_parameter(text: &str) -> Result<Vec<ImageEntry>, String> {
    let marker = "mtdparts=";
    let start = text
        .find(marker)
        .ok_or_else(|| "未找到 mtdparts 分区定义".to_owned())?;
    let definition = &text[start + marker.len()..];
    let definition = definition
        .split_whitespace()
        .next()
        .unwrap_or(definition)
        .trim_matches(|character| character == '\'' || character == '"');
    let partitions = definition
        .split_once(':')
        .map(|(_, value)| value)
        .unwrap_or(definition);

    let mut entries = Vec::new();
    for item in partitions.split(',') {
        let item = item.trim();
        let open = item
            .find('(')
            .ok_or_else(|| format!("无法解析分区项：{item}"))?;
        let close = item[open + 1..]
            .find(')')
            .map(|index| index + open + 1)
            .ok_or_else(|| format!("分区名缺少右括号：{item}"))?;
        let name = item[open + 1..close].trim();
        let before_name = &item[..open];
        let address = before_name
            .split_once('@')
            .map(|(_, value)| value.trim())
            .ok_or_else(|| format!("分区缺少起始地址：{item}"))?;
        if name.is_empty() || address.is_empty() {
            return Err(format!("分区名或地址为空：{item}"));
        }
        entries.push(ImageEntry::new(false, address, name));
    }

    if entries.is_empty() {
        Err("分区表为空".to_owned())
    } else {
        Ok(entries)
    }
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
