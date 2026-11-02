fn main() {
    let application = rktools_backend::create_application();
    if let Err(error) = rktools_frontend::desktop::run(application) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
