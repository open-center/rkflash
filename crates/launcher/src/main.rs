// Windows 发布版作为 GUI 程序启动，不附带控制台窗口。
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    let application = rkflash_backend::create_application();
    if let Err(error) = rkflash_frontend::desktop::run(application) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
