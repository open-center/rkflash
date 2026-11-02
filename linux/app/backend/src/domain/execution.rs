//! 后端内部执行计划；不会暴露给前端。

use std::ffi::OsString;

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

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub name: String,
    pub commands: Vec<CommandSpec>,
}

impl ExecutionPlan {
    pub fn single(name: impl Into<String>, command: CommandSpec) -> Self {
        Self {
            name: name.into(),
            commands: vec![command],
        }
    }
}
