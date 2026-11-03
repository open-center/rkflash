use super::FlashApplication;
use rkflash_api::{DeviceApi, DeviceSnapshot};

impl DeviceApi for FlashApplication {
    fn refresh_devices(&mut self) -> Option<DeviceSnapshot> {
        let snapshot = self.device_monitor.latest_device_snapshot()?;
        // 记住最近一次结果，设备操作执行前据此确认只连接了一台设备。
        self.device_snapshot = snapshot.clone();
        Some(snapshot)
    }
}
