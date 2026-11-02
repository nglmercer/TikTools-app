fn main() {
    let mut args = vec!["stage-native".to_owned()];
    args.extend(std::env::args().skip(1));
    if let Err(error) = tiktools_plugin_devkit::run(args) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
