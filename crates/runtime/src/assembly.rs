//! 把平台提供的烧录工具适配器与操作系统宿主装配成完整的应用。

use crate::host::OsHost;
use crate::persistence::{JsonConfigurationRepository, LocalFileSystem};
use crate::runtime::ProcessRuntime;
use rkflash_application::{
    ApplicationPorts, FlashApplication, FlashPlanner, RockchipImageProfile, ToolProtocol,
};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// 一个平台的烧录工具：命令规划、输出协议，以及工具路径与日志约定。
pub struct ToolBackend<A> {
    pub adapter: A,
    pub tool_path: Option<PathBuf>,
    /// 工具在 HOME 下写内部日志的固定相对路径。
    pub tool_log_relative_path: PathBuf,
}

pub fn assemble_application<A>(backend: ToolBackend<A>, host: Arc<dyn OsHost>) -> FlashApplication
where
    A: FlashPlanner + ToolProtocol + Clone + 'static,
{
    // 协议解析在后台线程运行，需要 Arc + Send/Sync；命令规划只在 UI 线程使用。
    let protocol: Arc<dyn ToolProtocol> = Arc::new(backend.adapter.clone());
    let planner: Rc<dyn FlashPlanner> = Rc::new(backend.adapter);
    let runtime = Rc::new(ProcessRuntime::new(
        backend.tool_path,
        protocol,
        backend.tool_log_relative_path,
        host,
    ));

    FlashApplication::new(ApplicationPorts {
        executor: runtime.clone(),
        device_monitor: runtime.clone(),
        tool_locator: runtime.clone(),
        operation_log: runtime,
        planner,
        profile: Rc::new(RockchipImageProfile),
        workspace_repository: Box::new(JsonConfigurationRepository),
        file_system: Box::new(LocalFileSystem),
    })
}
