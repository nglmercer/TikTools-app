mod args;
mod dev;
mod index_cmd;
mod keys;
mod pack_cmd;
mod scaffold;
mod stage_native;
mod targets;
mod validate;

#[cfg(test)]
mod tests;

pub use scaffold::TEMPLATES;

const USAGE: &str = "\
tiktools-plugin <command> [args]

commands:
  new <id> [--template process-rust|native-rust|napi-vm-ts|declarative|webview-ui] [--name text] [--publisher name] [--dir path]
  validate [dir|plugin.json] [--publish] [--legacy-id]
  build [dir] [--target <plugin-target> | --all-targets] [--release] [--out-dir dist] [--sign key.json]
  pack [dir] [--entry file] [--out-dir dist] [--target <plugin-target>] [--sign key.json]
  pack --manifest plugin.json --entry file --output file.plugin [--target <plugin-target>]
  stage-native --package <name> --source <dir> --plugin-dir <dir> --root <rel> [--overwrite]
  stage-native --provider [npm|github|all] --manifest plugin.json [--plugin-dir dir] [--pin] ...
  keygen --id <key-id> [--publisher name] [--role root|index|publisher] [--out dir]
  sign <file.plugin>... --key key.json
  verify <file.plugin> [--trust public-key.json]...
  index init|delegate|add|yank|advise|revoke|rebuild <registry-dir> ...
  publish <file.plugin>... --registry <registry-dir> --index-key key.json [--channel stable|beta] [--base-url url]
  dev stage|watch [dir] --into <dev-plugins-dir>";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let Some((command, rest)) = args.split_first() else {
        println!("{USAGE}");
        return Ok(());
    };
    match command.as_str() {
        "new" => scaffold::run(rest),
        "validate" => validate::run(rest),
        "build" => pack_cmd::build_command(rest),
        "pack" => pack_cmd::pack_command(rest),
        "stage-native" => stage_native::run(rest.to_vec()),
        "keygen" => keys::keygen(rest),
        "sign" => keys::sign(rest),
        "verify" => keys::verify(rest),
        "index" => index_cmd::index(rest),
        "publish" => index_cmd::publish(rest),
        "dev" => dev::run(rest),
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown command `{other}`\n{USAGE}").into()),
    }
}
