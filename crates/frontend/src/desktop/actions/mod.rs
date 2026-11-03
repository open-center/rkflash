//! 前端动作层：页面与外壳只调用这里的方法，这里是前端唯一调用后端 API 的地方。
//!
//! 子模块与 rkflash-api 的模块一一对应（`download.rs` 只调用 `DownloadApi`，依此类推）。
//! 会改变设备的操作分两步：`request_*` 先调用 API 的 `check_*` 校验并弹出确认框，
//! 用户确认后再由 [`RkFlashApp::run_confirmed`] 调用对应的执行函数。

mod advanced;
mod device;
mod download;
mod log;
mod settings;
mod task;
mod upgrade;
mod workspace;

use super::*;

/// 按钮因任务运行或平台不支持而禁用时的提示。
pub(super) const UNSUPPORTED_HINT: &str = "任务运行中，或当前平台的烧录工具不支持该操作";

impl RkFlashApp {
    /// 校验通过才弹出确认框；校验失败直接提示，不让用户确认一个注定失败的操作。
    fn ask_confirmation(
        &mut self,
        check: Result<(), String>,
        title: impl Into<String>,
        detail: impl Into<String>,
        danger: bool,
        action: ConfirmedAction,
    ) {
        if let Err(error) = check {
            self.report_error("操作校验失败", error);
            return;
        }
        self.pending_action = Some(PendingAction {
            title: title.into(),
            detail: detail.into(),
            danger,
            action,
        });
    }

    pub(super) fn run_confirmed(&mut self, action: ConfirmedAction) {
        match action {
            ConfirmedAction::Download(request) => self.download_images(request),
            ConfirmedAction::UpgradeFirmware(request) => self.upgrade_firmware(request),
            ConfirmedAction::EraseFlash(request) => self.erase_flash(request),
            ConfirmedAction::ControlDevice(control) => self.control_device(control),
            ConfirmedAction::UpgradeLoader(request) => self.upgrade_loader(request),
            ConfirmedAction::WriteLba(request) => self.write_lba(request),
            ConfirmedAction::EraseLba(request) => self.erase_lba(request),
        }
    }

    /// 后台任务未能启动时统一提示。
    fn report_start(&mut self, result: Result<(), String>) {
        if let Err(error) = result {
            self.report_error("无法开始操作", error);
        }
    }
}
