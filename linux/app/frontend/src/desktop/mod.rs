//! RKFlash 原生 egui 桌面交付层。

mod actions;
mod chrome;
mod components;
mod file_dialog;
mod layout;
mod overlays;
mod pages;
mod platform;
mod theme;
mod view_model;

use components::{
    button_grid, card, danger_button, field, format_bytes, inset, metric, path_picker,
    primary_button, section_title, stat_strip, status_badge,
};
use eframe::egui::{self, Align, Color32, Layout, RichText, Stroke};
use file_dialog::{FileDialogSpec, FileDialogTarget};
use layout::LayoutProfile;
use rkflash_api::{
    ApplicationUpdate, DeviceControl, DeviceQuery, DeviceSnapshot, ExecutionEvent, FlashOperation,
    FlashService, ImageEntry, StorageType, WorkspaceConfig, WORKSPACE_FORMAT_VERSION,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use view_model::{AppTab, DeviceMode, ErrorToast, LogEntry, LogLevel, OperationTip, PendingAction};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_ID: &str = "rkflash";

fn app_icon() -> eframe::egui::IconData {
    let image = image::load_from_memory(include_bytes!(
        "../../../../packaging/icons/rkflash-256.png"
    ))
    .expect("embedded application icon must be a valid PNG")
    .into_rgba8();
    let (width, height) = image.dimensions();
    eframe::egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

pub fn run(application: impl FlashService + 'static) -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            // Keep the Wayland app_id aligned with packaging/rkflash.desktop so
            // GNOME can group the running window with its launcher icon.
            .with_app_id(APP_ID)
            .with_title("RKFlash")
            .with_inner_size([1320.0, 860.0])
            .with_min_inner_size([960.0, 700.0])
            .with_icon(app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "RKFlash",
        options,
        Box::new(move |context| Ok(Box::new(RkFlashApp::new(context, Box::new(application))))),
    )
    .map_err(|error| format!("启动 RKFlash 图形界面失败：{error}"))
}

pub struct RkFlashApp {
    application: Box<dyn FlashService>,
    egui_ctx: egui::Context,
    file_dialogs: Vec<(FileDialogTarget, mpsc::Receiver<Option<PathBuf>>)>,
    tab: AppTab,
    images: Vec<ImageEntry>,
    selected_image: Option<usize>,
    firmware_path: String,
    firmware_size_cache: Option<(String, String)>,
    loader_path: String,
    tool_log_directory: String,
    output_path: String,
    lba_start: String,
    lba_count: String,
    lba_image_path: String,
    reset_after_download: bool,
    no_reset_after_upgrade: bool,
    loader_storage: usize,
    device_snapshot: DeviceSnapshot,
    logs: Vec<LogEntry>,
    progress: f32,
    status_text: String,
    pending_action: Option<PendingAction>,
    recognized_device_mode: Option<DeviceMode>,
    operation_tip: Option<OperationTip>,
    error_toast: Option<ErrorToast>,
    log_persistence_error: Option<String>,
    session_started_at: Instant,
    follow_logs: bool,
    logs_expanded: bool,
}

impl RkFlashApp {
    pub fn new(context: &eframe::CreationContext<'_>, application: Box<dyn FlashService>) -> Self {
        let cjk_font_available = theme::install(&context.egui_ctx);
        let tool_path = application.tool_path();
        let tool_available = tool_path.is_some();
        let tool_message = match tool_path {
            Some(path) => format!("已找到工具：{}", path.display()),
            None => "未找到 upgrade_tool，请从右上角选择官方工具".to_owned(),
        };
        let tool_log_directory = application.tool_log_directory().display().to_string();

        let mut app = Self {
            images: application.default_images(),
            application,
            egui_ctx: context.egui_ctx.clone(),
            file_dialogs: Vec::new(),
            tab: AppTab::Download,
            selected_image: None,
            firmware_path: String::new(),
            firmware_size_cache: None,
            loader_path: String::new(),
            tool_log_directory,
            output_path: String::new(),
            lba_start: "0x00000000".to_owned(),
            lba_count: "0x00002000".to_owned(),
            lba_image_path: String::new(),
            reset_after_download: true,
            no_reset_after_upgrade: false,
            loader_storage: 0,
            device_snapshot: DeviceSnapshot::default(),
            logs: Vec::new(),
            progress: 0.0,
            status_text: "就绪".to_owned(),
            pending_action: None,
            recognized_device_mode: None,
            operation_tip: None,
            error_toast: None,
            log_persistence_error: None,
            session_started_at: Instant::now(),
            follow_logs: true,
            logs_expanded: true,
        };
        app.push_log(LogLevel::Info, format!("RKFlash v{APP_VERSION} 已启动"));
        app.push_log(
            if tool_available {
                LogLevel::Success
            } else {
                LogLevel::Warning
            },
            tool_message,
        );
        if !cjk_font_available {
            app.push_log(
                LogLevel::Warning,
                "未找到系统中文字体（建议安装 fonts-noto-cjk），界面中文可能显示为方块",
            );
        }
        app
    }

    fn process_application(&mut self) {
        let ApplicationUpdate {
            device_snapshot,
            events,
        } = self.application.poll();
        if let Some(snapshot) = device_snapshot {
            self.update_device_operation_tip(&snapshot);
            self.device_snapshot = snapshot;
        }

        for event in events {
            match event {
                ExecutionEvent::TaskStarted { name, steps } => {
                    self.progress = 0.0;
                    self.status_text = format!("正在执行：{name}");
                    if let Some(path) = self.application.operation_log_path() {
                        self.push_log(LogLevel::Info, format!("操作日志：{}", path.display()));
                    }
                    self.push_log(LogLevel::Info, format!("开始{name}，共 {steps} 个步骤"));
                }
                ExecutionEvent::StepStarted {
                    index,
                    total,
                    label,
                    command,
                } => {
                    self.status_text = format!("[{index}/{total}] {label}");
                    self.push_log(LogLevel::Command, format!("$ {command}"));
                }
                ExecutionEvent::Output { text, stderr } => {
                    let lower = text.to_ascii_lowercase();
                    let level = if stderr || lower.contains("fail") || lower.contains("error") {
                        LogLevel::Error
                    } else if lower.contains("success") || lower.ends_with(" ok.") {
                        LogLevel::Success
                    } else {
                        LogLevel::Info
                    };
                    self.push_log(level, text);
                }
                ExecutionEvent::Progress(value) => self.progress = value,
                ExecutionEvent::TaskFinished {
                    name,
                    success,
                    cancelled,
                    message,
                } => {
                    self.progress = if success { 1.0 } else { self.progress };
                    self.status_text = if success {
                        format!("{name}完成")
                    } else if cancelled {
                        format!("{name}已中止")
                    } else {
                        format!("{name}未完成")
                    };
                    let level = if success {
                        LogLevel::Success
                    } else if cancelled {
                        LogLevel::Warning
                    } else {
                        LogLevel::Error
                    };
                    self.push_log(level, format!("{name}：{message}"));
                    if !success && !cancelled {
                        self.show_error_toast(format!("{name}失败"), message);
                    }
                }
            }
        }
    }

    fn push_log(&mut self, level: LogLevel, text: impl Into<String>) {
        let text = text.into();
        let persistence_result = self.application.append_operation_log(level.label(), &text);
        let elapsed = self.session_started_at.elapsed();
        self.logs.push(LogEntry {
            level,
            text,
            elapsed,
        });
        if let Err(error) = persistence_result {
            if self.log_persistence_error.as_deref() != Some(error.as_str()) {
                self.logs.push(LogEntry {
                    level: LogLevel::Error,
                    text: format!("操作日志未能写入磁盘：{error}"),
                    elapsed: self.session_started_at.elapsed(),
                });
                self.log_persistence_error = Some(error);
            }
        }
        if self.logs.len() > 2_000 {
            self.logs.drain(..500);
        }
    }

    pub(super) fn report_error(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        let title = title.into();
        let detail = detail.into();
        self.push_log(LogLevel::Error, format!("{title}：{detail}"));
        self.show_error_toast(title, detail);
    }

    fn show_error_toast(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        self.error_toast = Some(ErrorToast::new(title, detail));
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

    fn current_device_mode(&self) -> Option<DeviceMode> {
        match self.device_snapshot.devices.as_slice() {
            [device] => DeviceMode::from_label(&device.mode),
            _ => None,
        }
    }

    fn operation_log_display(&self) -> Option<(String, String)> {
        self.application.operation_log_path().map(|path| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            (name, path.display().to_string())
        })
    }

    /// 升级页每帧都要展示固件大小；按路径缓存结果，避免每帧一次磁盘 stat。
    fn cached_firmware_size(&mut self) -> String {
        let path = self.firmware_path.trim().to_owned();
        if let Some((cached_path, size)) = &self.firmware_size_cache {
            if *cached_path == path {
                return size.clone();
            }
        }
        let size = self
            .application
            .file_size(Path::new(&path))
            .map(format_bytes)
            .unwrap_or_else(|_| "—".to_owned());
        self.firmware_size_cache = Some((path, size.clone()));
        size
    }
}

impl eframe::App for RkFlashApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(context);
        self.process_application();
        self.poll_file_dialogs();
        self.render_header(context);
        self.render_status(context);
        self.render_logs(context);

        let layout = LayoutProfile::from_width(context.available_rect().width());
        // 确认对话框期间禁用背景交互，形成真正的模态。
        let modal_confirmation = self.pending_action.is_some();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::CANVAS)
                    .inner_margin(egui::Margin::symmetric(layout.content_margin, 18)),
            )
            .show(context, |ui| {
                if modal_confirmation {
                    ui.disable();
                }
                match self.tab {
                    AppTab::Download => self.render_download(ui),
                    AppTab::Upgrade => self.render_upgrade(ui),
                    AppTab::Advanced => self.render_advanced(ui),
                }
            });

        self.render_error_toast(context);
        self.render_operation_tip(context);
        self.render_confirmation(context);
        context.request_repaint_after(if self.application.is_active() {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(500)
        });
    }
}
