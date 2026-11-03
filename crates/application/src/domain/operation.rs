//! 后端内部的统一操作命令：api 中各功能区的请求在这里汇合，交给烧录工具适配器规划。

use rkflash_api::{
    DeviceControl, DeviceQuery, DownloadRequest, EraseFlashRequest, FirmwareUpgradeRequest,
    LbaEraseRequest, LbaReadRequest, LbaWriteRequest, LoaderUpgradeRequest, SerialLogRequest,
    StorageType, ToolInfo,
};
use rkflash_config::ImageEntry;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlashOperation {
    DownloadImages {
        images: Vec<ImageEntry>,
        reset_after: bool,
    },
    UpgradeFirmware {
        firmware: PathBuf,
        no_reset: bool,
        storage: StorageType,
    },
    EraseFlash {
        firmware: PathBuf,
    },
    Query {
        query: DeviceQuery,
    },
    Control(DeviceControl),
    UpgradeLoader {
        loader: PathBuf,
        storage: StorageType,
    },
    ReadLba {
        start: String,
        count: String,
        output: PathBuf,
    },
    WriteLba {
        start: String,
        image: PathBuf,
    },
    EraseLba {
        start: String,
        count: String,
    },
    ExportSerialLog {
        output: PathBuf,
    },
}

impl FlashOperation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::DownloadImages { .. } => "下载镜像",
            Self::UpgradeFirmware { .. } => "升级固件",
            Self::EraseFlash { .. } => "擦除 Flash",
            Self::Query { query, .. } => query.name(),
            Self::Control(control) => control.name(),
            Self::UpgradeLoader { .. } => "升级 Loader",
            Self::ReadLba { .. } => "读取 LBA",
            Self::WriteLba { .. } => "写入 LBA",
            Self::EraseLba { .. } => "擦除 LBA",
            Self::ExportSerialLog { .. } => "导出串口日志",
        }
    }

    /// 按工具声明的能力判断该操作能否执行。
    pub fn is_supported_by(&self, tool: &ToolInfo) -> bool {
        match self {
            Self::Query { query } => tool.queries.contains(query),
            Self::Control(control) => tool.controls.contains(control),
            Self::UpgradeLoader { storage, .. } => tool.loader_storages.contains(storage),
            Self::UpgradeFirmware { storage, .. } => {
                tool.firmware_storages.is_empty() || tool.firmware_storages.contains(storage)
            }
            Self::ExportSerialLog { .. } => tool.serial_log,
            _ => true,
        }
    }
}

impl From<DownloadRequest> for FlashOperation {
    fn from(request: DownloadRequest) -> Self {
        Self::DownloadImages {
            images: request.images,
            reset_after: request.reset_after,
        }
    }
}

impl From<FirmwareUpgradeRequest> for FlashOperation {
    fn from(request: FirmwareUpgradeRequest) -> Self {
        Self::UpgradeFirmware {
            firmware: request.firmware,
            no_reset: request.no_reset,
            storage: request.storage,
        }
    }
}

impl From<EraseFlashRequest> for FlashOperation {
    fn from(request: EraseFlashRequest) -> Self {
        Self::EraseFlash {
            firmware: request.firmware,
        }
    }
}

impl From<DeviceQuery> for FlashOperation {
    fn from(query: DeviceQuery) -> Self {
        Self::Query { query }
    }
}

impl From<DeviceControl> for FlashOperation {
    fn from(control: DeviceControl) -> Self {
        Self::Control(control)
    }
}

impl From<LoaderUpgradeRequest> for FlashOperation {
    fn from(request: LoaderUpgradeRequest) -> Self {
        Self::UpgradeLoader {
            loader: request.loader,
            storage: request.storage,
        }
    }
}

impl From<LbaReadRequest> for FlashOperation {
    fn from(request: LbaReadRequest) -> Self {
        Self::ReadLba {
            start: request.start,
            count: request.count,
            output: request.output,
        }
    }
}

impl From<LbaWriteRequest> for FlashOperation {
    fn from(request: LbaWriteRequest) -> Self {
        Self::WriteLba {
            start: request.start,
            image: request.image,
        }
    }
}

impl From<LbaEraseRequest> for FlashOperation {
    fn from(request: LbaEraseRequest) -> Self {
        Self::EraseLba {
            start: request.start,
            count: request.count,
        }
    }
}

impl From<SerialLogRequest> for FlashOperation {
    fn from(request: SerialLogRequest) -> Self {
        Self::ExportSerialLog {
            output: request.output,
        }
    }
}
