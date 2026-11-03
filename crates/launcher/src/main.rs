// Windows 发布版作为 GUI 程序启动，不附带控制台窗口。
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(not(windows))]
use rkflash_platform_linux as platform;
#[cfg(windows)]
use rkflash_platform_windows as platform;

fn main() {
    let application = platform::create_application();
    if let Err(error) = rkflash_frontend::desktop::run(application, platform::desktop_profile()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
