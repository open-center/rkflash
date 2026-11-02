//! rkdeveloptool 的多步工作流：下载镜像、update.img 整包升级与擦除 Flash。
//!
//! 整包升级流程与 macOS 版保持一致：校验 MD5 → 解包 → 核对芯片 → 进入 Maskrom →
//! 下载包内 Loader → 写入 IDBlock → 写入分区表 → 按固件清单地址写入各分区。

use crate::chip::{chip_code_from_chip_info, chip_compatibility, chip_display_name};
use crate::firmware::{self, FirmwareInfo, Md5Status, Progress};
use crate::partition::PartitionMetadata;
use crate::rkdeveloptool::{parse_device_list, Storage, UsbDevice};
use crate::session::{ToolSession, Workflow};
use std::cell::RefCell;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MODE_SWITCH_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// 下载列表中的一项：已知的 Rockchip 分区按名称写入，其余按起始扇区写入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadTarget {
    Partition(String),
    Sector(OsString),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadItem {
    pub name: String,
    pub target: DownloadTarget,
    pub path: PathBuf,
}

fn args<const N: usize>(items: [&dyn AsRef<std::ffi::OsStr>; N]) -> Vec<OsString> {
    items
        .iter()
        .map(|item| item.as_ref().to_os_string())
        .collect()
}

fn list_devices(session: &mut dyn ToolSession) -> Result<Vec<UsbDevice>, String> {
    // 没有设备时 ld 以非零状态退出，这里只关心解析结果。
    let outcome = session.probe(&args([&"ld"]))?;
    Ok(parse_device_list(&outcome.output))
}

fn single_device(session: &mut dyn ToolSession) -> Result<UsbDevice, String> {
    let mut devices = list_devices(session)?;
    match devices.len() {
        0 => Err("未检测到 Rockchip 设备，请确认开发板已进入 Loader 或 Maskrom 模式".to_owned()),
        1 => Ok(devices.remove(0)),
        count => Err(format!("检测到 {count} 台设备，请只保留一台再操作")),
    }
}

fn wait_for_mode(session: &mut dyn ToolSession, mode: &str) -> Result<UsbDevice, String> {
    let deadline = Instant::now() + MODE_SWITCH_TIMEOUT;
    while Instant::now() < deadline {
        if session.is_cancelled() {
            return Err("操作已由用户中止".to_owned());
        }
        std::thread::sleep(POLL_INTERVAL);
        let devices = list_devices(session)?;
        if devices.len() > 1 {
            return Err(format!(
                "检测到 {} 台设备，请只保留一台再操作",
                devices.len()
            ));
        }
        if let Some(device) = devices.into_iter().find(|device| device.is_mode(mode)) {
            session.status(&format!("设备已重新枚举为 {}", device.mode));
            return Ok(device);
        }
    }
    Err(format!("等待设备进入 {mode} 模式超时"))
}

/// 按内容识别 GPT 镜像或 parameter 并写入分区表；不是分区表时返回 `false`。
fn write_partition_metadata(
    session: &mut dyn ToolSession,
    name: &str,
    path: &Path,
) -> Result<bool, String> {
    let declared = matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "gpt" | "parameter"
    );
    match PartitionMetadata::inspect(path)? {
        PartitionMetadata::GptImage | PartitionMetadata::GptParameter => {
            session.status("识别为 GPT 分区表，正在写入主 GPT 和备份 GPT");
            session
                .run("写入 GPT 分区表", &args([&"gpt", &path]))?
                .require("写入 GPT 分区表")?;
            session
                .run("读取分区表", &args([&"ppt"]))?
                .require("读取分区表")?;
            Ok(true)
        }
        PartitionMetadata::LegacyParameter => {
            session.status("识别为传统 parameter 布局，正在写入 parameter 区域");
            session
                .run("写入 parameter", &args([&"prm", &path]))?
                .require("写入 parameter")?;
            Ok(true)
        }
        PartitionMetadata::UnknownParameter => {
            Err("无法识别 parameter 的分区布局（缺少 TYPE 或 mtdparts）".to_owned())
        }
        PartitionMetadata::Other if declared => {
            Err(format!("“{name}”的内容既不是 GPT 也不是 parameter"))
        }
        PartitionMetadata::Other => Ok(false),
    }
}

fn is_partition_table(name: &str, path: &Path) -> bool {
    PartitionMetadata::inspect(path).is_ok_and(PartitionMetadata::is_partition_table)
        || matches!(
            name.trim().to_ascii_lowercase().as_str(),
            "gpt" | "parameter"
        )
}

/// 解包使用的临时目录；任务结束时（无论成败）自动删除，避免残留数 GB 的镜像。
struct WorkDirectory(PathBuf);

impl WorkDirectory {
    fn new(label: &str) -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        Self(
            std::env::temp_dir()
                .join("rkflash")
                .join(format!("{label}-{nonce}-{sequence}")),
        )
    }
}

impl Drop for WorkDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 在共享会话上同时提供进度回调和取消检查。
fn with_progress<T>(
    session: &mut dyn ToolSession,
    start: f32,
    end: f32,
    action: impl FnOnce(Progress<'_>) -> Result<T, String>,
) -> Result<T, String> {
    let cell = RefCell::new(session);
    let mut report = |done: u64, total: u64| {
        if total > 0 {
            let fraction = done as f32 / total as f32;
            cell.borrow_mut().progress(start + (end - start) * fraction);
        }
    };
    let cancelled = || cell.borrow().is_cancelled();
    action(Progress {
        report: &mut report,
        cancelled: &cancelled,
    })
}

pub struct DownloadWorkflow {
    items: Vec<DownloadItem>,
    reset_after: bool,
}

impl DownloadWorkflow {
    pub fn new(mut items: Vec<DownloadItem>, reset_after: bool) -> Self {
        // 分区表必须先写入，按名称写入的分区才能找到位置。
        items.sort_by_key(|item| !is_partition_table(&item.name, &item.path));
        Self { items, reset_after }
    }
}

impl Workflow for DownloadWorkflow {
    fn estimated_steps(&self) -> usize {
        self.items.len() + usize::from(self.reset_after)
    }

    fn run(self: Box<Self>, session: &mut dyn ToolSession) -> Result<String, String> {
        single_device(session)?;
        let count = self.items.len() as f32;
        for (index, item) in self.items.iter().enumerate() {
            session.progress_range(index as f32 / count, (index + 1) as f32 / count);
            if write_partition_metadata(session, &item.name, &item.path)? {
                continue;
            }
            let label = format!("写入 {}", item.name);
            let command = match &item.target {
                DownloadTarget::Partition(partition) => {
                    args([&"wlx", &partition.as_str(), &item.path])
                }
                DownloadTarget::Sector(sector) => args([&"wl", sector, &item.path]),
            };
            session.run(&label, &command)?.require(&label)?;
            session.progress((index + 1) as f32 / count);
        }
        if self.reset_after {
            session
                .run("重启设备", &args([&"rd"]))?
                .require("重启设备")?;
        }
        Ok("镜像下载完成".to_owned())
    }
}

pub struct FirmwareUpgradeWorkflow {
    info: FirmwareInfo,
    storage: Storage,
    reset_after: bool,
}

impl FirmwareUpgradeWorkflow {
    pub fn new(info: FirmwareInfo, storage: Storage, reset_after: bool) -> Self {
        Self {
            info,
            storage,
            reset_after,
        }
    }

    fn check_chip(&self, session: &mut dyn ToolSession) -> Result<(), String> {
        let output = session.run("读取芯片信息", &args([&"rci"]))?.output;
        let firmware_name = self.info.chip_name();
        match (self.info.chip_code, chip_code_from_chip_info(&output)) {
            (Some(firmware), Some(device)) => {
                let device_name = chip_display_name(device);
                match chip_compatibility(firmware, device) {
                    Some(false) => Err(format!(
                        "固件芯片 {firmware_name} 与设备芯片 {device_name} 不匹配，已停止升级"
                    )),
                    Some(true) => {
                        session.status(&format!(
                            "芯片核对通过：固件 {firmware_name}，设备 {device_name}"
                        ));
                        Ok(())
                    }
                    None => {
                        session.status(&format!(
                            "警告：固件 {firmware_name}，设备 {device_name}，编码格式不同，无法自动严格比对"
                        ));
                        Ok(())
                    }
                }
            }
            _ => {
                session.status("警告：固件或设备未提供可比较的芯片编码，请人工核对型号");
                Ok(())
            }
        }
    }
}

impl Workflow for FirmwareUpgradeWorkflow {
    fn estimated_steps(&self) -> usize {
        6 + self.info.partitions.len() + usize::from(self.reset_after)
    }

    fn run(self: Box<Self>, session: &mut dyn ToolSession) -> Result<String, String> {
        let info = &self.info;
        session.status(&format!(
            "固件：{} {}，芯片 {}，Loader {}",
            info.model,
            info.firmware_version.as_deref().unwrap_or("未知版本"),
            info.chip_name(),
            info.loader_version.as_deref().unwrap_or("未知")
        ));

        session.status("正在校验固件 MD5");
        match with_progress(session, 0.0, 0.15, |progress| {
            firmware::verify_md5(info, progress)
        })? {
            Md5Status::Valid => session.status("固件 MD5 校验通过"),
            Md5Status::NotPresent => session.status("固件未附带 MD5，跳过校验"),
            Md5Status::Invalid => return Err("固件 MD5 校验失败，文件可能已损坏".to_owned()),
        }

        session.status("正在解包固件");
        let work = WorkDirectory::new("firmware");
        let extraction = with_progress(session, 0.15, 0.3, |progress| {
            firmware::extract(info, &work.0, progress)
        })?;
        let loader = extraction.loader_path.ok_or("固件包内没有可用的 Loader")?;
        if extraction.items.is_empty() {
            return Err("固件包内没有可烧录的分区".to_owned());
        }

        let mut device = single_device(session)?;
        self.check_chip(session)?;
        if device.is_mode("Loader") {
            session
                .run("切换到 Maskrom", &args([&"rd", &"3"]))?
                .require("切换到 Maskrom")?;
            device = wait_for_mode(session, "Maskrom")?;
        }
        if !device.is_mode("Maskrom") {
            return Err(format!(
                "设备处于 {} 模式，需要 Loader 或 Maskrom 模式",
                device.mode
            ));
        }

        let storage = self.storage.code();
        // 已下载过 Loader 的设备仍可能报告为 Maskrom，重复 db 可能卡住 USB：
        // 先探测能否切换存储，只对真正的 BootROM Maskrom 下载 Loader。
        let session_ready = match storage {
            Some(code) => session
                .probe(&args([&"cs", &code]))?
                .output
                .to_ascii_lowercase()
                .contains("change storage ok"),
            None => false,
        };
        if session_ready {
            session.status("检测到已就绪的 Loader 会话，跳过重复下载");
        } else {
            session
                .run("下载包内 Loader", &args([&"db", &loader]))?
                .require("下载包内 Loader")?;
            if let Some(code) = storage {
                session
                    .run("选择存储", &args([&"cs", &code]))?
                    .require("选择存储")?;
            }
        }
        session
            .run("写入 IDBlock", &args([&"ul", &loader]))?
            .require("写入 IDBlock")?;
        // Loader 下载后 USB 仍报告 Maskrom；在同一会话内继续写入，重启会丢失该会话。
        if let Some(code) = storage {
            session
                .run("确认存储通道", &args([&"cs", &code]))?
                .require("确认存储通道")?;
        }
        session.progress(0.35);

        let mut items = extraction.items;
        items.sort_by_key(|item| !is_partition_table(&item.partition_name, &item.image_path));
        let count = items.len() as f32;
        for (index, item) in items.iter().enumerate() {
            let start = 0.35 + 0.65 * index as f32 / count;
            session.progress_range(start, 0.35 + 0.65 * (index + 1) as f32 / count);
            if write_partition_metadata(session, &item.partition_name, &item.image_path)? {
                continue;
            }
            if item.address == u64::from(u32::MAX) {
                return Err(format!("固件清单中“{}”的地址无效", item.partition_name));
            }
            let label = format!("写入 {}", item.partition_name);
            let address = format!("0x{:08X}", item.address);
            session
                .run(&label, &args([&"wl", &address, &item.image_path]))?
                .require(&label)?;
        }
        session.progress(1.0);

        if self.reset_after {
            session
                .run("重启设备", &args([&"rd"]))?
                .require("重启设备")?;
        }
        Ok("固件升级完成；全程使用包内 Loader".to_owned())
    }
}

pub struct EraseFlashWorkflow {
    info: FirmwareInfo,
}

impl EraseFlashWorkflow {
    pub fn new(info: FirmwareInfo) -> Self {
        Self { info }
    }
}

impl Workflow for EraseFlashWorkflow {
    fn estimated_steps(&self) -> usize {
        2
    }

    fn run(self: Box<Self>, session: &mut dyn ToolSession) -> Result<String, String> {
        let device = single_device(session)?;
        if device.is_mode("Maskrom") {
            let work = WorkDirectory::new("erase");
            let loader = firmware::extract_loader(&self.info, &work.0)?;
            session
                .run("下载包内 Loader", &args([&"db", &loader]))?
                .require("下载包内 Loader")?;
        }
        session
            .run("擦除 Flash", &args([&"ef"]))?
            .require("擦除 Flash")?;
        Ok("Flash 擦除完成".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::ToolOutcome;
    use std::collections::VecDeque;

    /// 按顺序返回预设输出的会话，记录工作流实际调用的命令。
    struct ScriptedSession {
        responses: VecDeque<(&'static str, &'static str)>,
        commands: Vec<String>,
    }

    impl ScriptedSession {
        fn reply(&mut self, args: &[OsString]) -> ToolOutcome {
            let command = args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" ");
            let (expected, output) = self.responses.pop_front().expect("unexpected command");
            assert!(command.starts_with(expected), "{command} != {expected}");
            self.commands.push(command);
            ToolOutcome {
                success: true,
                failure: None,
                output: output.to_owned(),
            }
        }
    }

    impl ToolSession for ScriptedSession {
        fn run(&mut self, _label: &str, args: &[OsString]) -> Result<ToolOutcome, String> {
            Ok(self.reply(args))
        }
        fn probe(&mut self, args: &[OsString]) -> Result<ToolOutcome, String> {
            Ok(self.reply(args))
        }
        fn status(&mut self, _text: &str) {}
        fn progress_range(&mut self, _start: f32, _end: f32) {}
        fn progress(&mut self, _value: f32) {}
        fn is_cancelled(&self) -> bool {
            false
        }
    }

    #[test]
    fn download_writes_partition_table_first_and_uses_names() {
        let directory = std::env::temp_dir().join("rkflash-core-download-test");
        std::fs::create_dir_all(&directory).unwrap();
        let parameter = directory.join("parameter.txt");
        std::fs::write(
            &parameter,
            "TYPE: GPT\nCMDLINE: mtdparts=:-@0x4000(rootfs)\n",
        )
        .unwrap();
        std::fs::write(directory.join("boot.img"), b"\x7fBOOTIMAGE").unwrap();
        let workflow = Box::new(DownloadWorkflow::new(
            vec![
                DownloadItem {
                    name: "boot".to_owned(),
                    target: DownloadTarget::Partition("boot".to_owned()),
                    path: directory.join("boot.img"),
                },
                DownloadItem {
                    name: "parameter".to_owned(),
                    target: DownloadTarget::Partition("parameter".to_owned()),
                    path: parameter,
                },
            ],
            true,
        ));
        let mut session = ScriptedSession {
            responses: VecDeque::from([
                ("ld", "DevNo=1\tVid=0x2207,Pid=0x350b,LocationID=1\tLoader"),
                ("gpt", ""),
                ("ppt", ""),
                ("wlx boot", ""),
                ("rd", ""),
            ]),
            commands: Vec::new(),
        };
        workflow.run(&mut session).unwrap();
        assert!(session.responses.is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn download_refuses_multiple_devices() {
        let workflow = Box::new(DownloadWorkflow::new(Vec::new(), false));
        let mut session = ScriptedSession {
            responses: VecDeque::from([(
                "ld",
                "DevNo=1\tVid=0x2207,Pid=0x350b,LocationID=1\tLoader\nDevNo=2\tVid=0x2207,Pid=0x350b,LocationID=2\tLoader",
            )]),
            commands: Vec::new(),
        };
        assert!(workflow.run(&mut session).unwrap_err().contains("2 台设备"));
    }
}
