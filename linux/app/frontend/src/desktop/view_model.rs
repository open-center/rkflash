use rktools_api::FlashOperation;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AppTab {
    Download,
    Upgrade,
    Advanced,
}

#[derive(Clone)]
pub(super) struct PendingAction {
    pub title: String,
    pub detail: String,
    pub danger: bool,
    pub operation: FlashOperation,
}

#[derive(Clone, Copy)]
pub(super) enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
    Command,
}

impl LogLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Success => "SUCCESS",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
            Self::Command => "COMMAND",
        }
    }
}

pub(super) struct LogEntry {
    pub level: LogLevel,
    pub text: String,
    pub elapsed: Duration,
}

impl LogEntry {
    pub fn elapsed_label(&self) -> String {
        let total_millis = self.elapsed.as_millis();
        let minutes = total_millis / 60_000;
        let seconds = (total_millis / 1_000) % 60;
        let millis = total_millis % 1_000;
        format!("+{minutes:02}:{seconds:02}.{millis:03}")
    }
}

pub(super) struct ErrorToast {
    pub title: String,
    pub detail: String,
    timer: NoticeTimer,
}

impl ErrorToast {
    pub fn new(title: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            detail: detail.into(),
            timer: NoticeTimer::new(Duration::from_secs(15)),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.timer.is_expired()
    }

    pub fn remaining_fraction(&self) -> f32 {
        self.timer.remaining_fraction()
    }
}

struct NoticeTimer {
    shown_at: Instant,
    duration: Duration,
}

impl NoticeTimer {
    fn new(duration: Duration) -> Self {
        Self {
            shown_at: Instant::now(),
            duration,
        }
    }

    fn is_expired(&self) -> bool {
        self.shown_at.elapsed() >= self.duration
    }

    fn remaining_fraction(&self) -> f32 {
        1.0 - (self.shown_at.elapsed().as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DeviceMode {
    Maskrom,
    Loader,
}

pub(super) struct DeviceModeGuidance {
    pub title: &'static str,
    pub detail: &'static str,
}

impl DeviceMode {
    pub fn from_label(mode: &str) -> Option<Self> {
        if mode.eq_ignore_ascii_case("maskrom") {
            Some(Self::Maskrom)
        } else if mode.eq_ignore_ascii_case("loader") {
            Some(Self::Loader)
        } else {
            None
        }
    }

    pub fn guidance(self) -> DeviceModeGuidance {
        match self {
            Self::Maskrom => DeviceModeGuidance {
                title: "设备已进入 Maskrom 模式",
                detail:
                    "这是底层恢复模式。rktools 只显示当前状态，不会主动加载 Loader 或切换模式。",
            },
            Self::Loader => DeviceModeGuidance {
                title: "设备已进入 Loader 模式",
                detail: "设备通信已就绪，可以直接读取设备信息、下载镜像或升级固件。",
            },
        }
    }
}

pub(super) struct OperationTip {
    pub mode: DeviceMode,
    timer: NoticeTimer,
}

impl OperationTip {
    pub fn new(mode: DeviceMode) -> Self {
        Self {
            mode,
            timer: NoticeTimer::new(Duration::from_secs(10)),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.timer.is_expired()
    }

    pub fn remaining_fraction(&self) -> f32 {
        self.timer.remaining_fraction()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_operation_tips_only_for_supported_device_modes() {
        let maskrom = DeviceMode::from_label("Maskrom").expect("Maskrom mode");
        assert_eq!(maskrom.guidance().title, "设备已进入 Maskrom 模式");

        let loader = DeviceMode::from_label("LOADER").expect("Loader mode");
        assert_eq!(loader.guidance().title, "设备已进入 Loader 模式");

        assert!(DeviceMode::from_label("U-Boot").is_none());
    }

    #[test]
    fn creates_error_toast_and_stable_log_labels() {
        let toast = ErrorToast::new("烧录失败", "设备已断开");
        assert_eq!(toast.title, "烧录失败");
        assert_eq!(toast.detail, "设备已断开");
        assert!(!toast.is_expired());
        assert!(toast.remaining_fraction() > 0.0);
        assert_eq!(LogLevel::Error.label(), "ERROR");
        assert_eq!(LogLevel::Command.label(), "COMMAND");
    }

    #[test]
    fn formats_log_elapsed_time_for_scanning() {
        let entry = LogEntry {
            level: LogLevel::Info,
            text: "ready".to_owned(),
            elapsed: Duration::from_millis(62_345),
        };
        assert_eq!(entry.elapsed_label(), "+01:02.345");
    }
}
