fn main() {
    if let Err(error) = tiktools_plugin_devkit::run(std::env::args().skip(1).collect()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
