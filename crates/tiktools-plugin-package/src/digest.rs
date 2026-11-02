use std::{
    fmt,
    fs::File,
    io::{self, Read},
    path::Path,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Digest(String);

impl Sha256Digest {
    pub fn parse(value: &str) -> Option<Self> {
        let hex = value.strip_prefix("sha256:").unwrap_or(value);
        (hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .then(|| Self(hex.to_ascii_lowercase()))
    }

    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256Hasher::new();
        hasher.update(bytes);
        hasher.finish()
    }

    pub fn of_reader(mut reader: impl Read) -> io::Result<Self> {
        let mut hasher = Sha256Hasher::new();
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                return Ok(hasher.finish());
            }
            hasher.update(&buffer[..read]);
        }
    }

    pub fn of_file(path: &Path) -> io::Result<Self> {
        Self::of_reader(File::open(path)?)
    }

    pub fn as_hex(&self) -> &str {
        &self.0
    }

    pub fn prefixed(&self) -> String {
        format!("sha256:{}", self.0)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        (0..self.0.len())
            .step_by(2)
            .filter_map(|index| u8::from_str_radix(&self.0[index..index + 2], 16).ok())
            .collect()
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for Sha256Digest {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value).ok_or_else(|| format!("invalid sha256 digest: {value}"))
    }
}

impl From<Sha256Digest> for String {
    fn from(value: Sha256Digest) -> Self {
        value.0
    }
}

#[derive(Default)]
pub struct Sha256Hasher(Sha256);

impl Sha256Hasher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    pub fn finish(self) -> Sha256Digest {
        Sha256Digest(
            self.0
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
        )
    }
}
