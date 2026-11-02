fn main() {
    let application = rkflash_backend::create_application();
    if let Err(error) = rkflash_frontend::desktop::run(application) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
