//! 通用外部进程运行时。

mod command;
mod logging;
mod monitor;
mod runtime;
mod terminal;

pub(crate) use runtime::ProcessRuntime;
