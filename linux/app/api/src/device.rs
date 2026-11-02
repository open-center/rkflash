//! USB 枚举产生的稳定设备领域状态。

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
