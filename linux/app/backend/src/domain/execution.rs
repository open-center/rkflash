//! 后端内部执行计划；不会暴露给前端。

use rkflash_core::session::Workflow;
use std::ffi::OsString;
use std::fmt;

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub label: String,
    pub args: Vec<OsString>,
}

impl CommandSpec {
    pub fn new(
        label: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<OsString>>,
    ) -> Self {
        Self {
            label: label.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }
}

pub struct ExecutionPlan {
    pub name: String,
    pub commands: Vec<CommandSpec>,
    /// 存在时由运行时执行工作流，`commands` 不再使用。
    pub workflow: Option<Box<dyn Workflow>>,
}

impl ExecutionPlan {
    pub fn new(name: impl Into<String>, commands: Vec<CommandSpec>) -> Self {
        Self {
            name: name.into(),
            commands,
            workflow: None,
        }
    }

    pub fn single(name: impl Into<String>, command: CommandSpec) -> Self {
        Self::new(name, vec![command])
    }

    pub fn workflow(name: impl Into<String>, workflow: Box<dyn Workflow>) -> Self {
        Self {
            name: name.into(),
            commands: Vec::new(),
            workflow: Some(workflow),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty() && self.workflow.is_none()
    }
}

impl fmt::Debug for ExecutionPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExecutionPlan")
            .field("name", &self.name)
            .field("commands", &self.commands)
            .field("workflow", &self.workflow.is_some())
            .finish()
    }
}
