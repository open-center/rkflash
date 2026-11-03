//! api 各功能区接口的后端实现：每个子模块实现 rkflash-api 中的同名接口，
//! 系统能力全部经应用端口注入。

mod advanced;
mod device;
mod download;
mod log;
mod settings;
mod task;
mod upgrade;
mod workspace;

use super::{
    validation::validate_operation, DeviceMonitor, FileSystem, FlashPlanner, FlashProfile,
    OperationExecutor, OperationLog, ToolLocator, WorkspaceRepository,
};
use crate::domain::{DeviceSnapshot, FlashOperation};
use std::rc::Rc;

pub struct ApplicationPorts {
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
    pub fn new(ports: ApplicationPorts) -> Self {
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

    /// 所有设备操作共用的执行前校验：工具、能力、唯一设备与输入文件。
    fn check(&self, operation: &FlashOperation) -> Result<(), String> {
        if self.tool_locator.tool_path().is_none() {
            return Err("未配置烧录工具，请点击右上角“选择工具”".to_owned());
        }
        let tool = self.planner.tool_info();
        if !operation.is_supported_by(&tool) {
            return Err(format!("{} 不支持“{}”", tool.name, operation.name()));
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

    /// 所有设备操作共用的执行入口：校验、规划、开启操作日志并交给执行器。
    fn run(&self, operation: FlashOperation) -> Result<(), String> {
        self.check(&operation)?;
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
}
