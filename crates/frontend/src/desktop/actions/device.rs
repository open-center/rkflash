//! 设备状态 ↔ `DeviceApi`。

use super::super::*;

impl RkFlashApp {
    pub(in crate::desktop) fn refresh_devices(&mut self) {
        if let Some(snapshot) = self.application.refresh_devices() {
            self.update_device_operation_tip(&snapshot);
            self.device_snapshot = snapshot;
        }
    }

    fn update_device_operation_tip(&mut self, snapshot: &DeviceSnapshot) {
        let mode = match snapshot.devices.as_slice() {
            [device] => DeviceMode::from_label(&device.mode),
            _ => None,
        };
        if mode == self.recognized_device_mode {
            return;
        }

        self.operation_tip = mode.map(OperationTip::new);
        self.recognized_device_mode = mode;
    }
}
