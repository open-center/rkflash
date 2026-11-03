//! RKFlash 原生 egui 桌面交付层。

mod actions;
mod chrome;
mod components;
mod file_dialog;
mod font;
mod layout;
mod overlays;
mod pages;
mod profile;
mod theme;
mod view_model;

pub use profile::{show_error_dialog, DesktopProfile, Renderer};

use components::{
    button_grid, card, danger_button, field, format_bytes, inset, metric, path_picker,
    primary_button, section_title, stat_strip, status_badge,
};
use eframe::egui::{self, Align, Color32, Layout, RichText, Stroke};
use file_dialog::{FileDialogSpec, FileDialogTarget};
use layout::LayoutProfile;
use rkflash_api::{
    DeviceControl, DeviceQuery, DeviceSnapshot, DownloadRequest, EraseFlashRequest, ExecutionEvent,
    FirmwareUpgradeRequest, FlashService, ImageEntry, LbaEraseRequest, LbaReadRequest,
    LbaWriteRequest, LoaderUpgradeRequest, SerialLogRequest, StorageType, ToolInfo,
    WorkspaceConfig, WORKSPACE_FORMAT_VERSION,
};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use view_model::{
    AppTab, ConfirmedAction, DeviceMode, ErrorToast, LogEntry, LogLevel, OperationTip,
    PendingAction,
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_ID: &str = "rkflash";

fn app_icon() -> eframe::egui::IconData {
    let image = image::load_from_memory(include_bytes!("../../../../assets/icons/rkflash-256.png"))
        .expect("embedded application icon must be a valid PNG")
        .into_rgba8();
    let (width, height) = image.dimensions();
    eframe::egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

pub fn run(
    application: impl FlashService + 'static,
    profile: DesktopProfile,
) -> Result<(), String> {
    // 首选渲染器创建失败时（例如虚拟机或远程桌面没有可用的 OpenGL 2+），
    // 换下一个渲染器重试；应用对象只有在窗口真正创建后才会被取走。
    let slot: Rc<RefCell<Option<Box<dyn FlashService>>>> =
        Rc::new(RefCell::new(Some(Box::new(application))));
    let mut failures = Vec::new();
    let cjk_fonts = profile.cjk_fonts;
    for &(renderer, label) in profile.renderers {
        let pending = Rc::clone(&slot);
        let result = eframe::run_native(
            "RKFlash",
            native_options(renderer),
            Box::new(move |context| {
                let application = pending.borrow_mut().take().ok_or("界面已启动")?;
                Ok(Box::new(RkFlashApp::new(context, application, cjk_fonts)))
            }),
        );
        match result {
            Ok(()) => return Ok(()),
            Err(error) => failures.push(format!("{label}：{error}")),
        }
        if slot.borrow().is_none() {
            break;
        }
    }
    let message = format!("启动 RKFlash 图形界面失败。{}", failures.join("；"));
    (profile.show_startup_error)(&message);
    Err(message)
}

fn native_options(renderer: eframe::Renderer) -> eframe::NativeOptions {
    eframe::NativeOptions {
        renderer,
        viewport: eframe::egui::ViewportBuilder::default()
            // Keep the Wayland app_id aligned with packaging/rkflash.desktop so
            // GNOME can group the running window with its launcher icon.
            .with_app_id(APP_ID)
            .with_title("RKFlash")
            .with_inner_size([1320.0, 860.0])
            .with_min_inner_size([960.0, 700.0])
            .with_icon(app_icon()),
        ..Default::default()
    }
}

pub struct RkFlashApp {
    /// 只允许 `actions/` 访问；页面与外壳经动作层调用后端。
    application: Box<dyn FlashService>,
    /// 当前平台烧录工具的能力，启动时读取一次。
    tool: ToolInfo,
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
    firmware_storage: usize,
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
    pub fn new(
        context: &eframe::CreationContext<'_>,
        application: Box<dyn FlashService>,
        cjk_fonts: &[(&str, u32)],
    ) -> Self {
        let cjk_font_available = theme::install(&context.egui_ctx, cjk_fonts);
        let mut app = Self {
            application,
            tool: ToolInfo::default(),
            egui_ctx: context.egui_ctx.clone(),
            file_dialogs: Vec::new(),
            tab: AppTab::Download,
            images: Vec::new(),
            selected_image: None,
            firmware_path: String::new(),
            firmware_size_cache: None,
            loader_path: String::new(),
            tool_log_directory: String::new(),
            output_path: String::new(),
            lba_start: "0x00000000".to_owned(),
            lba_count: "0x00002000".to_owned(),
            lba_image_path: String::new(),
            reset_after_download: true,
            no_reset_after_upgrade: false,
            loader_storage: 0,
            firmware_storage: 0,
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
        app.load_tool_settings();
        app.restore_default_images();
        if !cjk_font_available {
            app.push_log(
                LogLevel::Warning,
                "未找到系统中文字体（建议安装 fonts-noto-cjk），界面中文可能显示为方块",
            );
        }
        app
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

    fn current_device_mode(&self) -> Option<DeviceMode> {
        match self.device_snapshot.devices.as_slice() {
            [device] => DeviceMode::from_label(&device.mode),
            _ => None,
        }
    }
}

impl eframe::App for RkFlashApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(context);
        self.refresh_devices();
        self.process_task_events();
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
        context.request_repaint_after(if self.is_task_running() {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(500)
        });
    }
}
