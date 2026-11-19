//! Startup mode parsing.
//!
//! Remote control is owned by the installable `tiktools.remote-control`
//! plugin. The desktop host always starts as the local host; the controller
//! window and `--controller`/`--remote-profile` selection were removed with
//! the remote-client crate.

#[derive(Debug, PartialEq)]
pub enum Mode {
    Local,
}

impl Mode {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, &'static str> {
        let unknown = args
            .into_iter()
            .find(|arg| arg.starts_with("--controller") || arg.starts_with("--remote-profile"));
        match unknown {
            None => Ok(Self::Local),
            Some(_) => Err("remote controller support is provided by the remote-control plugin"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_is_the_only_mode() {
        assert_eq!(Mode::parse(["--some-host-option".to_owned()]), Ok(Mode::Local));
    }

    #[test]
    fn controller_flags_are_rejected_with_a_plugin_hint() {
        assert!(Mode::parse(["--controller".to_owned()]).is_err());
        assert!(Mode::parse(["--remote-profile=phone".to_owned()]).is_err());
    }
}
