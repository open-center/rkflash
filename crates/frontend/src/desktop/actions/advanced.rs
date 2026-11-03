//! 高级功能页 ↔ `AdvancedApi`。

use super::super::*;

impl RkFlashApp {
    pub(in crate::desktop) fn query_device(&mut self, query: DeviceQuery) {
        let result = self.application.query_device(query);
        self.report_start(result);
    }

    /// 会改变设备状态的控制先确认，其余直接执行。
    pub(in crate::desktop) fn request_control_device(
        &mut self,
        control: DeviceControl,
        confirm: bool,
    ) {
        if confirm {
            self.ask_confirmation(
                self.application.check_control_device(control),
                control.name(),
                "此操作会改变设备当前状态，是否继续？",
                false,
                ConfirmedAction::ControlDevice(control),
            );
        } else {
            self.control_device(control);
        }
    }

    pub(super) fn control_device(&mut self, control: DeviceControl) {
        let result = self.application.control_device(control);
        self.report_start(result);
    }

    pub(in crate::desktop) fn request_upgrade_loader(&mut self, storage: StorageType) {
        let request = LoaderUpgradeRequest {
            loader: PathBuf::from(&self.loader_path),
            storage,
        };
        self.ask_confirmation(
            self.application.check_upgrade_loader(&request),
            "确认升级 Loader",
            "Loader 必须与芯片和存储类型匹配。写入错误 Loader 可能导致设备只能通过 Maskrom 恢复。",
            true,
            ConfirmedAction::UpgradeLoader(request),
        );
    }

    pub(super) fn upgrade_loader(&mut self, request: LoaderUpgradeRequest) {
        let result = self.application.upgrade_loader(request);
        self.report_start(result);
    }

    pub(in crate::desktop) fn read_lba(&mut self, output: PathBuf) {
        let result = self.application.read_lba(LbaReadRequest {
            start: self.lba_start.clone(),
            count: self.lba_count.clone(),
            output,
        });
        self.report_start(result);
    }

    pub(in crate::desktop) fn request_write_lba(&mut self) {
        let request = LbaWriteRequest {
            start: self.lba_start.clone(),
            image: PathBuf::from(&self.lba_image_path),
        };
        self.ask_confirmation(
            self.application.check_write_lba(&request),
            "确认写入 LBA",
            "将从指定扇区开始写入镜像，请再次核对起始地址。",
            true,
            ConfirmedAction::WriteLba(request),
        );
    }

    pub(super) fn write_lba(&mut self, request: LbaWriteRequest) {
        let result = self.application.write_lba(request);
        self.report_start(result);
    }

    pub(in crate::desktop) fn request_erase_lba(&mut self) {
        let request = LbaEraseRequest {
            start: self.lba_start.clone(),
            count: self.lba_count.clone(),
        };
        self.ask_confirmation(
            self.application.check_erase_lba(&request),
            "确认擦除 LBA",
            "指定范围的数据将被永久擦除。",
            true,
            ConfirmedAction::EraseLba(request),
        );
    }

    pub(super) fn erase_lba(&mut self, request: LbaEraseRequest) {
        let result = self.application.erase_lba(request);
        self.report_start(result);
    }

    pub(in crate::desktop) fn export_serial_log(&mut self) {
        let result = self.application.export_serial_log(SerialLogRequest {
            output: PathBuf::from(&self.output_path),
        });
        self.report_start(result);
    }
}
