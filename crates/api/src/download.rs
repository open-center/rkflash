//! 下载镜像页：分区下载列表、parameter 导入与按分区写入。

use crate::ImageEntry;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadRequest {
    pub images: Vec<ImageEntry>,
    pub reset_after: bool,
}

pub trait DownloadApi {
    /// 当前平台的默认下载列表。
    fn default_images(&self) -> Vec<ImageEntry>;
    /// 解析 parameter.txt，并在同目录匹配各分区的固件文件。
    fn import_parameter(&self, path: &Path) -> Result<Vec<ImageEntry>, String>;
    /// 读取设备当前分区表，结果通过任务事件输出。
    fn read_partition_table(&self) -> Result<(), String>;
    fn check_download(&self, request: &DownloadRequest) -> Result<(), String>;
    fn download_images(&self, request: DownloadRequest) -> Result<(), String>;
}
