use super::FlashApplication;
use crate::domain::FlashOperation;
use rkflash_api::{
    AdvancedApi, DeviceControl, DeviceQuery, LbaEraseRequest, LbaReadRequest, LbaWriteRequest,
    LoaderUpgradeRequest, SerialLogRequest,
};

impl AdvancedApi for FlashApplication {
    fn query_device(&self, query: DeviceQuery) -> Result<(), String> {
        self.run(query.into())
    }

    fn check_control_device(&self, control: DeviceControl) -> Result<(), String> {
        self.check(&control.into())
    }

    fn control_device(&self, control: DeviceControl) -> Result<(), String> {
        self.run(control.into())
    }

    fn check_upgrade_loader(&self, request: &LoaderUpgradeRequest) -> Result<(), String> {
        self.check(&FlashOperation::from(request.clone()))
    }

    fn upgrade_loader(&self, request: LoaderUpgradeRequest) -> Result<(), String> {
        self.run(request.into())
    }

    fn read_lba(&self, request: LbaReadRequest) -> Result<(), String> {
        self.run(request.into())
    }

    fn check_write_lba(&self, request: &LbaWriteRequest) -> Result<(), String> {
        self.check(&FlashOperation::from(request.clone()))
    }

    fn write_lba(&self, request: LbaWriteRequest) -> Result<(), String> {
        self.run(request.into())
    }

    fn check_erase_lba(&self, request: &LbaEraseRequest) -> Result<(), String> {
        self.check(&FlashOperation::from(request.clone()))
    }

    fn erase_lba(&self, request: LbaEraseRequest) -> Result<(), String> {
        self.run(request.into())
    }

    fn export_serial_log(&self, request: SerialLogRequest) -> Result<(), String> {
        self.run(request.into())
    }
}
