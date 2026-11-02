//! `FlashService` 的后端实现；所有系统能力都经应用端口注入。

use super::{
    image_resolution::resolve_image_paths, validation::validate_operation, DeviceMonitor,
    FileSystem, FlashPlanner, FlashProfile, OperationExecutor, OperationLog, ToolLocator,
    WorkspaceRepository,
};
use rkflash_api::{
    ApplicationUpdate, DeviceSnapshot, FlashOperation, FlashService, ImageEntry, WorkspaceConfig,
    WORKSPACE_FORMAT_VERSION,
};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub(crate) struct ApplicationPorts {
    pub executor: Rc<dyn OperationExecutor>,
    pub device_monitor: Rc<dyn DeviceMonitor>,
    pub tool_locator: Rc<dyn ToolLocator>,
    pub operation_log: Rc<dyn OperationLog>,
    pub planner: Rc<dyn FlashPlanner>,
    pub profile: Rc<dyn FlashProfile>,
    pub workspace_repository: Box<dyn WorkspaceRepository>,
    pub file_system: Box<dyn FileSystem>,
}

pub struct FlashApplication {
    executor: Rc<dyn OperationExecutor>,
    device_monitor: Rc<dyn DeviceMonitor>,
    tool_locator: Rc<dyn ToolLocator>,
    operation_log: Rc<dyn OperationLog>,
    planner: Rc<dyn FlashPlanner>,
    profile: Rc<dyn FlashProfile>,
    workspace_repository: Box<dyn WorkspaceRepository>,
    file_system: Box<dyn FileSystem>,
    device_snapshot: DeviceSnapshot,
}

impl FlashApplication {
    pub(crate) fn new(ports: ApplicationPorts) -> Self {
        Self {
            executor: ports.executor,
            device_monitor: ports.device_monitor,
            tool_locator: ports.tool_locator,
            operation_log: ports.operation_log,
            planner: ports.planner,
            profile: ports.profile,
            workspace_repository: ports.workspace_repository,
            file_system: ports.file_system,
            device_snapshot: DeviceSnapshot::default(),
        }
    }
}

impl FlashService for FlashApplication {
    fn poll(&mut self) -> ApplicationUpdate {
        let device_snapshot = self.device_monitor.latest_device_snapshot();
        if let Some(snapshot) = &device_snapshot {
            self.device_snapshot = snapshot.clone();
        }
        ApplicationUpdate {
            device_snapshot,
            events: self.executor.drain_events(),
        }
    }

    fn execute(&self, operation: FlashOperation) -> Result<(), String> {
        self.validate(&operation)?;
        if self.executor.is_active() {
            return Err("已有任务正在执行".to_owned());
        }
        let plan = self.planner.plan(&operation)?;
        let operation_name = operation.name();
        let log_path = self.operation_log.begin_operation_log(operation_name)?;
        if let Err(error) = self.executor.start(plan) {
            let _ = self
                .operation_log
                .append_operation_log("ERROR", &format!("{operation_name}未能启动：{error}"));
            self.operation_log.finish_operation_log();
            return Err(format!("{error}；操作日志：{}", log_path.display()));
        }
        Ok(())
    }

    fn validate(&self, operation: &FlashOperation) -> Result<(), String> {
        if self.tool_locator.tool_path().is_none() {
            return Err("未配置烧录工具，请点击右上角“选择工具”".to_owned());
        }
        match self.device_snapshot.devices.len() {
            0 => {
                // 扫描失败时把具体原因带出来，而不是只提示“没有设备”。
                return Err(match self.device_snapshot.error.as_deref() {
                    Some(error) => format!("没有发现可用设备：{error}"),
                    None => "没有发现可用设备，请检查连接、设备模式和访问权限".to_owned(),
                });
            }
            1 => {}
            count => {
                return Err(format!(
                    "发现 {count} 台设备。为避免误刷，要求只连接一台目标设备"
                ))
            }
        }
        validate_operation(operation, self.file_system.as_ref())
    }

    fn cancel(&self) {
        self.executor.cancel();
    }

    fn is_active(&self) -> bool {
        self.executor.is_active()
    }

    fn tool_path(&self) -> Option<PathBuf> {
        self.tool_locator.tool_path()
    }

    fn set_tool_path(&self, path: PathBuf) -> Result<(), String> {
        if !self.file_system.is_file(&path) {
            return Err(format!("工具文件不存在：{}", path.display()));
        }
        self.tool_locator.set_tool_path(Some(path));
        Ok(())
    }

    fn tool_log_directory(&self) -> PathBuf {
        self.operation_log.tool_log_directory()
    }

    fn set_tool_log_directory(&self, path: PathBuf) -> Result<(), String> {
        self.operation_log.set_tool_log_directory(path)
    }

    fn operation_log_path(&self) -> Option<PathBuf> {
        self.operation_log.operation_log_path()
    }

    fn append_operation_log(&self, level: &str, text: &str) -> Result<(), String> {
        self.operation_log.append_operation_log(level, text)
    }

    fn default_images(&self) -> Vec<ImageEntry> {
        self.profile.default_images()
    }

    fn file_size(&self, path: &Path) -> Result<u64, String> {
        self.file_system.file_size(path)
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

    fn load_workspace(&self, path: &Path) -> Result<WorkspaceConfig, String> {
        let config = self.workspace_repository.load_workspace(path)?;
        if config.format_version != WORKSPACE_FORMAT_VERSION {
            return Err(format!(
                "不支持的配置版本 {}，当前仅支持版本 {}",
                config.format_version, WORKSPACE_FORMAT_VERSION
            ));
        }
        Ok(config)
    }

    fn save_workspace(&self, path: &Path, config: &WorkspaceConfig) -> Result<(), String> {
        self.workspace_repository.save_workspace(path, config)
    }
}
