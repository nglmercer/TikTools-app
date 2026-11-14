//! Parse controller selection before logging, host locks, plugin install or GUI.
#[derive(Debug, PartialEq)]
pub enum Mode {
    Local,
    Controller(Option<String>),
}
impl Mode {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, &'static str> {
        let args: Vec<_> = args.into_iter().collect();
        if !args.iter().any(|s| {
            s == "--controller" || s == "--remote-profile" || s.starts_with("--remote-profile=")
        }) {
            return Ok(Self::Local);
        }
        let mut profile = None;
        let mut controller = false;
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            if arg == "--controller" && !controller {
                controller = true;
                continue;
            }
            let name = if arg == "--remote-profile" {
                iter.next().ok_or("remote profile name is required")?
            } else if let Some(name) = arg.strip_prefix("--remote-profile=") {
                name.into()
            } else {
                return Err(
                    "controller selection cannot be combined with host or installer options",
                );
            };
            if profile.is_some()
                || name.is_empty()
                || name.len() > 64
                || name.starts_with('-')
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            {
                return Err("invalid or duplicate remote profile selection");
            }
            profile = Some(name);
        }
        Ok(Self::Controller(profile))
    }
}
/// Preserve CLI arguments exactly; a remote profile belongs to the CLI here,
/// not the desktop controller mode. No environment or filesystem is touched.
pub fn cli_arguments(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Option<Vec<std::ffi::OsString>> {
    let mut args = args.into_iter();
    (args.next()?.as_os_str() == std::ffi::OsStr::new("--cli")).then(|| args.collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Mode, &'static str> {
        Mode::parse(args.iter().map(|s| s.to_string()))
    }
    #[test]
    fn packaged_cli_dispatch_precedes_desktop_remote_selection() {
        let args = [
            "--cli",
            "--remote-profile",
            "agent",
            "api",
            "call",
            "live.status",
            "{}",
        ];
        assert_eq!(
            cli_arguments(args.map(std::ffi::OsString::from)).unwrap(),
            args[1..]
                .iter()
                .map(|value| std::ffi::OsString::from(*value))
                .collect::<Vec<_>>()
        );
        assert!(cli_arguments(["--controller"].map(std::ffi::OsString::from)).is_none());
    }
    #[test]
    fn remote_selection_is_exclusive_before_host_initialization() {
        assert_eq!(parse(&["--controller"]), Ok(Mode::Controller(None)));
        assert_eq!(
            parse(&["--remote-profile=desk"]),
            Ok(Mode::Controller(Some("desk".into())))
        );
        for args in [
            vec!["--controller", "--install-plugin", "x.plugin"],
            vec!["--remote-profile", "--standalone"],
            vec!["--remote-profile=a", "--remote-profile=b"],
            vec!["--controller", "https://host#secret"],
        ] {
            assert!(parse(&args).is_err());
        }
    }
}
