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
