use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde_json::json;
use tiktools_plugin_api::manifest::{is_valid_plugin_id, is_valid_publisher};

use crate::args::{print_json, Args, CommandResult};

const USAGE: &str = "tiktools-plugin new <id> [--template process-rust|native-rust|napi-vm-ts|declarative|webview-ui] [--name \"Display Name\"] [--publisher name] [--dir path] [--sdk-path path]";

pub const TEMPLATES: [&str; 5] = [
    "process-rust",
    "native-rust",
    "napi-vm-ts",
    "declarative",
    "webview-ui",
];

const README: &str = include_str!("../templates/common/README.md.tmpl");
const CHANGELOG: &str = include_str!("../templates/common/CHANGELOG.md.tmpl");
const GITIGNORE: &str = include_str!("../templates/common/gitignore.tmpl");
const CARGO_TOML: &str = include_str!("../templates/rust/Cargo.toml.tmpl");
const PLUGIN_RS: &str = include_str!("../templates/rust/plugin.rs.tmpl");
const ACTION_MANIFEST: &str = include_str!("../templates/action/plugin.json.tmpl");
const NAPI_PACKAGE: &str = include_str!("../templates/napi-vm-ts/package.json.tmpl");
const NAPI_TSCONFIG: &str = include_str!("../templates/napi-vm-ts/tsconfig.json.tmpl");
const NAPI_INDEX: &str = include_str!("../templates/napi-vm-ts/index.ts.tmpl");
const DECLARATIVE_MANIFEST: &str = include_str!("../templates/declarative/plugin.json.tmpl");
const WEBVIEW_PACKAGE: &str = include_str!("../templates/webview-ui/package.json.tmpl");
const WEBVIEW_INDEX: &str = include_str!("../templates/webview-ui/index.html.tmpl");
const WEBVIEW_MAIN: &str = include_str!("../templates/webview-ui/main.ts.tmpl");

const PROCESS_ATTRIBUTES: &str = "#![forbid(unsafe_code)]\n#![cfg_attr(\n    all(target_os = \"windows\", not(debug_assertions)),\n    windows_subsystem = \"windows\"\n)]\n\n";
const WEBVIEW_UI: &str = "\n  \"ui\": {\n    \"apiVersion\": 1,\n    \"mode\": \"webview\",\n    \"entry\": \"ui/dist/index.html\",\n    \"pages\": [{ \"id\": \"main\", \"title\": { \"default\": \"{{name}}\" } }]\n  },";
const SDK_GIT: &str =
    "{ git = \"https://github.com/nglmercer/TikTools-app\", branch = \"remake\" }";

pub(crate) fn run(raw: &[String]) -> CommandResult {
    let args = Args::parse(raw);
    let id = args.positional(0).ok_or(USAGE)?.to_owned();
    if !is_valid_plugin_id(&id) {
        return Err(format!(
            "`{id}` is not a valid plugin id (lowercase letters, digits, `.`, `_`, `-`)"
        )
        .into());
    }
    let template = args.flag("--template").unwrap_or("process-rust");
    if !TEMPLATES.contains(&template) {
        return Err(format!(
            "unknown template `{template}`; expected one of {}",
            TEMPLATES.join(", ")
        )
        .into());
    }
    let publisher = args
        .flag("--publisher")
        .map(ToOwned::to_owned)
        .or_else(|| {
            id.split_once('.')
                .map(|(publisher, _)| publisher.to_owned())
        })
        .unwrap_or_else(|| "local".to_owned());
    if !is_valid_publisher(&publisher) {
        return Err(format!("`{publisher}` is not a valid publisher name").into());
    }
    let directory = args
        .path("--dir")
        .unwrap_or_else(|| PathBuf::from(id.replace('.', "-")));
    if directory.exists() && fs::read_dir(&directory)?.next().is_some() {
        return Err(format!("{} is not empty", directory.display()).into());
    }
    let sdk = match args.path("--sdk-path") {
        Some(path) => {
            if !path.join("Cargo.toml").is_file() {
                return Err(
                    format!("{} is not the tiktools-plugin-sdk crate", path.display()).into(),
                );
            }
            format!(
                "{{ path = \"{}\" }}",
                std::path::absolute(&path)?
                    .display()
                    .to_string()
                    .replace('\\', "/")
            )
        }
        None => SDK_GIT.to_owned(),
    };
    let files = render_template(template, &id, &publisher, args.flag("--name"), &sdk);
    for (relative, contents) in &files {
        let path = directory.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
    }
    print_json(&json!({
        "created": directory.display().to_string(),
        "template": template,
        "files": files.keys().collect::<Vec<_>>(),
        "next": next_steps(template, &directory),
    }));
    Ok(())
}

pub(crate) fn render_template(
    template: &str,
    id: &str,
    publisher: &str,
    display_name: Option<&str>,
    sdk: &str,
) -> BTreeMap<String, String> {
    let crate_name = id.replace(['.', '_'], "-");
    let name = display_name
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| title_case(&crate_name));
    let type_name = format!("{}Plugin", title_case(&crate_name).replace(' ', ""));
    let version = env!("CARGO_PKG_VERSION");
    let mut values = BTreeMap::from([
        ("id", id.to_owned()),
        ("name", name.clone()),
        ("manifest_name", name),
        ("publisher", publisher.to_owned()),
        ("crate", crate_name.clone()),
        ("type", type_name),
        ("template", template.to_owned()),
        ("engine", format!(">={version}")),
        ("sdk", sdk.to_owned()),
        ("npm_sdk", format!("^{version}")),
        ("npm_ui", format!("^{version}")),
        ("lib", String::new()),
        ("attributes", String::new()),
        ("export", String::new()),
        ("runtime", String::new()),
        ("entry", String::new()),
        ("runtime_fields", String::new()),
        ("category", "integrations".to_owned()),
        ("ui", String::new()),
    ]);
    let mut files = BTreeMap::from([
        ("README.md".to_owned(), README),
        ("CHANGELOG.md".to_owned(), CHANGELOG),
        (".gitignore".to_owned(), GITIGNORE),
    ]);
    match template {
        "process-rust" | "native-rust" => {
            let native = template == "native-rust";
            values.insert(
                "runtime",
                if native { "native" } else { "process" }.to_owned(),
            );
            values.insert(
                "entry",
                if native {
                    format!("native/{crate_name}.bin")
                } else {
                    crate_name.clone()
                },
            );
            if native {
                values.insert("lib", "\n[lib]\ncrate-type = [\"cdylib\"]\n".to_owned());
                values.insert("export", "tiktools_export_native_plugin".to_owned());
                values.insert(
                    "runtime_fields",
                    "\n  \"abiVersion\": 1,\n  \"trust\": \"trusted\",".to_owned(),
                );
            } else {
                values.insert("attributes", PROCESS_ATTRIBUTES.to_owned());
                values.insert("export", "tiktools_process_plugin".to_owned());
            }
            files.insert("Cargo.toml".to_owned(), CARGO_TOML);
            files.insert(
                if native { "src/lib.rs" } else { "src/main.rs" }.to_owned(),
                PLUGIN_RS,
            );
            files.insert("plugin.json".to_owned(), ACTION_MANIFEST);
        }
        "napi-vm-ts" => {
            values.insert("runtime", "napi-vm".to_owned());
            values.insert("entry", "dist/index.js".to_owned());
            values.insert("manifest_name", crate_name.clone());
            values.insert("runtime_fields", "\n  \"apiVersion\": 1,".to_owned());
            files.insert("package.json".to_owned(), NAPI_PACKAGE);
            files.insert("tsconfig.json".to_owned(), NAPI_TSCONFIG);
            files.insert("src/index.ts".to_owned(), NAPI_INDEX);
            files.insert("plugin.json".to_owned(), ACTION_MANIFEST);
        }
        "webview-ui" => {
            values.insert("category", "overlays".to_owned());
            values.insert("ui", WEBVIEW_UI.to_owned());
            files.insert("package.json".to_owned(), WEBVIEW_PACKAGE);
            files.insert("ui/index.html".to_owned(), WEBVIEW_INDEX);
            files.insert("ui/src/main.ts".to_owned(), WEBVIEW_MAIN);
            files.insert("plugin.json".to_owned(), DECLARATIVE_MANIFEST);
        }
        _ => {
            files.insert("plugin.json".to_owned(), DECLARATIVE_MANIFEST);
        }
    }
    files
        .into_iter()
        .map(|(path, template)| (path, render(template, &values)))
        .collect()
}

fn render(template: &str, values: &BTreeMap<&str, String>) -> String {
    let mut output = template.to_owned();
    for _ in 0..2 {
        for (key, value) in values {
            output = output.replace(&format!("{{{{{key}}}}}"), value);
        }
    }
    output
}

fn title_case(value: &str) -> String {
    value
        .split(['-', '_', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .map(|first| first.to_uppercase().chain(characters).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn next_steps(template: &str, directory: &Path) -> Vec<String> {
    let directory = directory.display();
    let mut steps = vec![format!("cd {directory}")];
    match template {
        "process-rust" | "native-rust" => steps.push("tiktools-plugin build".to_owned()),
        "napi-vm-ts" | "webview-ui" => {
            steps.push("bun install".to_owned());
            steps.push("tiktools-plugin build".to_owned());
        }
        _ => steps.push("tiktools-plugin pack".to_owned()),
    }
    steps.push("tiktools-plugin validate --publish".to_owned());
    steps
}
