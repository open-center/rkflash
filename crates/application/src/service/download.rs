use super::FlashApplication;
use crate::domain::FlashOperation;
use crate::image_resolution::resolve_image_paths;
use rkflash_api::{DeviceQuery, DownloadApi, DownloadRequest, ImageEntry};
use std::path::Path;

impl DownloadApi for FlashApplication {
    fn default_images(&self) -> Vec<ImageEntry> {
        self.profile.default_images()
    }

    fn import_parameter(&self, path: &Path) -> Result<Vec<ImageEntry>, String> {
        let text = self.file_system.read_text(path)?;
        let mut entries = self.profile.parse_parameter(&text)?;
        if !entries
            .iter()
            .any(|entry| entry.name.eq_ignore_ascii_case("parameter"))
        {
            entries.insert(0, ImageEntry::new(false, "0x00000000", "parameter"));
        }

        let directory = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let files = self.file_system.list_files(directory)?;
        resolve_image_paths(&mut entries, path, &files);
        Ok(entries)
    }

    fn read_partition_table(&self) -> Result<(), String> {
        self.run(DeviceQuery::PartitionTable.into())
    }

    fn check_download(&self, request: &DownloadRequest) -> Result<(), String> {
        self.check(&FlashOperation::from(request.clone()))
    }

    fn download_images(&self, request: DownloadRequest) -> Result<(), String> {
        self.run(request.into())
    }
}
