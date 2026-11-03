//! 设备状态：USB 枚举产生的设备列表与存储类型。

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub number: String,
    pub vid: String,
    pub pid: String,
    pub location: String,
    pub mode: String,
    pub serial: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceSnapshot {
    pub devices: Vec<DeviceInfo>,
    pub error: Option<String>,
}

/// 写入目标存储。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageType {
    Auto,
    Emmc,
    SpiNor,
    SpiNand,
}

impl StorageType {
    pub const ALL: [Self; 4] = [Self::Auto, Self::Emmc, Self::SpiNor, Self::SpiNand];

    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "自动",
            Self::Emmc => "eMMC",
            Self::SpiNor => "SPI NOR",
            Self::SpiNand => "SPI NAND",
        }
    }
}

pub trait DeviceApi {
    /// 取回后台轮询到的最新设备列表；自上次调用以来没有新结果时返回 `None`。
    fn refresh_devices(&mut self) -> Option<DeviceSnapshot>;
}
