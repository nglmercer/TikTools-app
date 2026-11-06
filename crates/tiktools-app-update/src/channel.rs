use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
}

impl Channel {
    pub fn permits(self, current: &Version, candidate: &Version) -> bool {
        candidate > current && (self == Self::Beta || candidate.pre.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn channels_reject_downgrades_and_stable_rejects_prereleases() {
        let v = |s| Version::parse(s).unwrap();
        assert!(!Channel::Stable.permits(&v("4.0.0"), &v("4.0.0")));
        assert!(!Channel::Beta.permits(&v("4.0.0"), &v("3.9.9")));
        assert!(!Channel::Stable.permits(&v("4.0.0"), &v("4.1.0-beta.1")));
        assert!(Channel::Beta.permits(&v("4.0.0"), &v("4.1.0-beta.1")));
        assert!(Channel::Beta.permits(&v("4.1.0-beta.1"), &v("4.1.0")));
    }
}
