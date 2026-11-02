use url::Url;

use crate::RegistryError;

pub fn is_allowed_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    match url.scheme() {
        "https" => url.host().is_some(),
        "http" => matches!(
            url.host_str(),
            Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
        ),
        _ => false,
    }
}

pub(crate) fn checked_url(value: &str) -> Result<Url, RegistryError> {
    if !is_allowed_url(value) {
        return Err(RegistryError::InsecureUrl(value.to_owned()));
    }
    Url::parse(value).map_err(|_| RegistryError::InsecureUrl(value.to_owned()))
}

pub fn join_url(base: &str, path: &str) -> Result<String, RegistryError> {
    if path.contains("://") {
        return checked_url(path).map(String::from);
    }
    let mut base = checked_url(base)?;
    if !base.path().ends_with('/') {
        let path = format!("{}/", base.path());
        base.set_path(&path);
    }
    let joined = base
        .join(path.trim_start_matches('/'))
        .map_err(|_| RegistryError::InsecureUrl(path.to_owned()))?;
    checked_url(joined.as_str()).map(String::from)
}
