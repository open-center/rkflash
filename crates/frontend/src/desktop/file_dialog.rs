//! rfd 文件对话框的后台线程执行。
//!
//! rfd 的同步 API 在 xdg-portal 后端上会阻塞调用线程等待 DBus 往返；
//! 直接在 UI 线程调用会冻结整个界面循环。这里统一在工作线程打开对话框，
//! 通过频道回传结果并唤醒 UI 线程。

use super::*;
use rfd::FileDialog;
use std::sync::mpsc;

/// 对话框类型与展示参数；在工作线程内转换为 `rfd::FileDialog`。
#[derive(Clone)]
pub(super) struct FileDialogSpec {
    pub title: Option<String>,
    pub filter_name: &'static str,
    pub filters: &'static [&'static str],
    pub file_name: Option<String>,
    pub directory: Option<PathBuf>,
    pub kind: FileDialogKind,
}

impl FileDialogSpec {
    pub(super) fn pick_file(
        title: &str,
        filter_name: &'static str,
        filters: &'static [&'static str],
    ) -> Self {
        Self {
            title: Some(title.to_owned()),
            filter_name,
            filters,
            file_name: None,
            directory: None,
            kind: FileDialogKind::PickFile,
        }
    }

    pub(super) fn save_file(
        title: &str,
        filter_name: &'static str,
        filters: &'static [&'static str],
        file_name: &str,
    ) -> Self {
        Self {
            title: Some(title.to_owned()),
            filter_name,
            filters,
            file_name: Some(file_name.to_owned()),
            directory: None,
            kind: FileDialogKind::SaveFile,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FileDialogKind {
    PickFile,
    SaveFile,
    PickFolder,
}

/// 对话框结果如何回写应用状态。
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum FileDialogTarget {
    ToolExecutable,
    ParameterFile,
    WorkspaceExport,
    WorkspaceImport,
    FirmwareImage,
    DownloadImage(usize),
    LoaderBinary,
    LbaImage,
    LbaExport,
    SerialLogOutput,
    ToolLogDirectory,
}

impl RkFlashApp {
    /// 在工作线程打开文件对话框；同一时刻允许多个待处理请求。
    pub(super) fn spawn_file_dialog(&mut self, spec: FileDialogSpec, target: FileDialogTarget) {
        let (sender, receiver) = mpsc::channel::<Option<PathBuf>>();
        let context = self.egui_ctx.clone();
        std::thread::spawn(move || {
            let mut dialog = FileDialog::new();
            if let Some(title) = spec.title.as_deref() {
                dialog = dialog.set_title(title);
            }
            if !spec.filters.is_empty() {
                dialog = dialog.add_filter(spec.filter_name, spec.filters);
            }
            if let Some(file_name) = spec.file_name.as_deref() {
                dialog = dialog.set_file_name(file_name);
            }
            if let Some(directory) = spec.directory.as_deref() {
                dialog = dialog.set_directory(directory);
            }
            let result = match spec.kind {
                FileDialogKind::PickFile => dialog.pick_file(),
                FileDialogKind::SaveFile => dialog.save_file(),
                FileDialogKind::PickFolder => dialog.pick_folder(),
            };
            let _ = sender.send(result);
            // 对话框关闭后立即唤醒 UI 线程处理结果。
            context.request_repaint();
        });
        self.file_dialogs.push((target, receiver));
    }

    pub(super) fn poll_file_dialogs(&mut self) {
        let mut finished = Vec::new();
        for (index, (target, receiver)) in self.file_dialogs.iter().enumerate() {
            if let Ok(path) = receiver.try_recv() {
                finished.push((index, target.clone(), path));
            }
        }
        // 逆序移除，避免索引失效；未选择任何文件时只清理请求。
        for (index, target, path) in finished.into_iter().rev() {
            self.file_dialogs.remove(index);
            if let Some(path) = path {
                self.apply_file_dialog_result(&target, path);
            }
        }
    }

    /// 后台文件对话框完成后的结果分发；只在用户确实选择了路径时调用。
    fn apply_file_dialog_result(&mut self, target: &FileDialogTarget, path: PathBuf) {
        match target {
            FileDialogTarget::ToolExecutable => self.set_tool_path(path),
            FileDialogTarget::ParameterFile => self.import_parameter(path),
            FileDialogTarget::WorkspaceExport => self.save_workspace(path),
            FileDialogTarget::WorkspaceImport => self.load_workspace(path),
            FileDialogTarget::FirmwareImage => {
                self.firmware_path = path.display().to_string();
            }
            FileDialogTarget::DownloadImage(index) => {
                if let Some(entry) = self.images.get_mut(*index) {
                    entry.path = path.display().to_string();
                    entry.enabled = true;
                }
            }
            FileDialogTarget::LoaderBinary => {
                self.loader_path = path.display().to_string();
            }
            FileDialogTarget::LbaImage => {
                self.lba_image_path = path.display().to_string();
            }
            FileDialogTarget::LbaExport => {
                self.lba_image_path = path.display().to_string();
                self.read_lba(path);
            }
            FileDialogTarget::SerialLogOutput => {
                self.output_path = path.display().to_string();
            }
            FileDialogTarget::ToolLogDirectory => self.set_tool_log_directory(path),
        }
    }
}
