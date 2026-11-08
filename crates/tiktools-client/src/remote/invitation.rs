//! Private invitation parsing shared by native frontends; errors never echo input.
use super::{fail, ClientError, SecretString};
use std::collections::BTreeMap;

pub fn parse_invitation(
    input: &str,
    expected_origin: &str,
) -> Result<(String, SecretString), ClientError> {
    if input.len() > 8192 {
        return Err(fail(
            "too_large",
            "private invitation input exceeds its limit",
        ));
    }
    super::http::origin(expected_origin)?;
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Invitation {
        invite_id: String,
        secret: String,
        #[serde(default)]
        expires_at: Option<i64>,
    }
    let (id, secret) = if input.trim_start().starts_with('{') {
        let value: Invitation = serde_json::from_str(input)
            .map_err(|_| fail("invalid_params", "invalid private invitation input"))?;
        let _ = value.expires_at;
        (value.invite_id, value.secret)
    } else {
        let url = url::Url::parse(input.trim())
            .map_err(|_| fail("invalid_params", "invalid private invitation link"))?;
        if url.origin().ascii_serialization() != expected_origin
            || url.path() != "/remote/v1/join"
            || url.query().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(fail(
                "invalid_params",
                "invitation does not belong to the selected HTTPS host",
            ));
        }
        let mut fields = BTreeMap::new();
        for (k, v) in url::form_urlencoded::parse(url.fragment().unwrap_or("").as_bytes()) {
            if !matches!(k.as_ref(), "inviteId" | "secret")
                || fields.insert(k.into_owned(), v.into_owned()).is_some()
            {
                return Err(fail("invalid_params", "invalid private invitation link"));
            }
        }
        (
            fields
                .remove("inviteId")
                .ok_or_else(|| fail("invalid_params", "invitation ID is required"))?,
            fields
                .remove("secret")
                .ok_or_else(|| fail("invalid_params", "invitation secret is required"))?,
        )
    };
    if id.len() != 36 || secret.len() != 64 || !secret.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(fail("invalid_params", "invalid private invitation input"));
    }
    Ok((id, SecretString::from(secret)))
}
