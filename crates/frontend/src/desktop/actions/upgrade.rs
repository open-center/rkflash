//! 升级固件页 ↔ `UpgradeApi`。

use super::super::*;

impl RkFlashApp {
    /// 升级页每帧都要展示固件大小；按路径缓存结果，避免每帧一次磁盘 stat。
    pub(in crate::desktop) fn cached_firmware_size(&mut self) -> String {
        let path = self.firmware_path.trim().to_owned();
        if let Some((cached_path, size)) = &self.firmware_size_cache {
            if *cached_path == path {
                return size.clone();
            }
        }
        let size = self
            .application
            .firmware_size(Path::new(&path))
            .map(format_bytes)
            .unwrap_or_else(|_| "—".to_owned());
        self.firmware_size_cache = Some((path, size.clone()));
        size
    }

    pub(in crate::desktop) fn request_upgrade_firmware(&mut self, storage: StorageType) {
        let request = FirmwareUpgradeRequest {
            firmware: PathBuf::from(&self.firmware_path),
            no_reset: self.no_reset_after_upgrade,
            storage,
        };
        self.ask_confirmation(
            self.application.check_upgrade_firmware(&request),
            "确认升级统一固件",
            "升级会重写设备分区。请确认固件型号与目标芯片匹配，并保持 USB 和电源稳定。",
            true,
            ConfirmedAction::UpgradeFirmware(request),
        );
    }

    pub(super) fn upgrade_firmware(&mut self, request: FirmwareUpgradeRequest) {
        let result = self.application.upgrade_firmware(request);
        self.report_start(result);
    }

    pub(in crate::desktop) fn request_erase_flash(&mut self) {
        let request = EraseFlashRequest {
            firmware: PathBuf::from(&self.firmware_path),
        };
        self.ask_confirmation(
            self.application.check_erase_flash(&request),
            "确认擦除 Flash",
            "此操作会清除目标存储上的全部数据，无法撤销。",
            true,
            ConfirmedAction::EraseFlash(request),
        );
    }

    pub(super) fn erase_flash(&mut self, request: EraseFlashRequest) {
        let result = self.application.erase_flash(request);
        self.report_start(result);
    }
}
