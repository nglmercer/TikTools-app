use std::path::PathBuf;

pub(crate) type CommandResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Default)]
pub(crate) struct Args {
    positional: Vec<String>,
    flags: Vec<(String, Option<String>)>,
}

const SWITCHES: [&str; 13] = [
    "--publish",
    "--release",
    "--yes",
    "--undo",
    "--all-targets",
    "--force",
    "--clear",
    "--help",
    "--legacy-id",
    "--once",
    "--json",
    "--overwrite",
    "--pin",
];

impl Args {
    pub(crate) fn parse(raw: &[String]) -> Self {
        let mut args = Self::default();
        let mut index = 0;
        while index < raw.len() {
            let value = &raw[index];
            if let Some(flag) = value.strip_prefix("--") {
                if let Some((name, inline)) = flag.split_once('=') {
                    args.flags
                        .push((format!("--{name}"), Some(inline.to_owned())));
                } else if SWITCHES.contains(&value.as_str()) {
                    args.flags.push((value.clone(), None));
                } else {
                    args.flags
                        .push((value.clone(), raw.get(index + 1).cloned()));
                    index += 1;
                }
            } else {
                args.positional.push(value.clone());
            }
            index += 1;
        }
        args
    }

    pub(crate) fn positional(&self, index: usize) -> Option<&str> {
        self.positional.get(index).map(String::as_str)
    }

    pub(crate) fn rest(&self, from: usize) -> &[String] {
        self.positional.get(from..).unwrap_or_default()
    }

    pub(crate) fn flag(&self, name: &str) -> Option<&str> {
        self.flags
            .iter()
            .rev()
            .find(|(flag, _)| flag == name)
            .and_then(|(_, value)| value.as_deref())
    }

    pub(crate) fn flags(&self, name: &str) -> Vec<String> {
        self.flags
            .iter()
            .filter(|(flag, _)| flag == name)
            .filter_map(|(_, value)| value.clone())
            .collect()
    }

    pub(crate) fn has(&self, name: &str) -> bool {
        self.flags.iter().any(|(flag, _)| flag == name)
    }

    pub(crate) fn path(&self, name: &str) -> Option<PathBuf> {
        self.flag(name).map(PathBuf::from)
    }

    pub(crate) fn required(&self, name: &str, usage: &str) -> Result<String, String> {
        self.flag(name)
            .map(ToOwned::to_owned)
            .ok_or_else(|| format!("{name} is required\n{usage}"))
    }
}

pub(crate) fn print_json(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    );
}
