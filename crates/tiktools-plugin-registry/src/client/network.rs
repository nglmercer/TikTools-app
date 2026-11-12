use super::*;

impl RegistryClient {
    pub(super) async fn cached_get(
        &self,
        source: &RegistrySource,
        path: &str,
    ) -> Result<Vec<u8>, RegistryError> {
        let url = join_url(&source.url, path)?;
        let local = if self.fixed_sources {
            self.cache_dir
                .join(&source.name)
                .join(Sha256Digest::of_bytes(source.url.as_bytes()).as_hex())
                .join(path)
        } else {
            self.cache_dir.join(&source.name).join(path)
        };
        let etag_path = local.with_extension(format!(
            "{}.etag",
            local
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
        ));
        let etag = fs::read_to_string(&etag_path)
            .ok()
            .filter(|_| local.is_file());
        match self.get(&url, etag.as_deref()).await {
            Ok(Some((bytes, etag))) => {
                write_cache(&local, &bytes)?;
                match etag {
                    Some(etag) => fs::write(&etag_path, etag)?,
                    None => {
                        let _ = fs::remove_file(&etag_path);
                    }
                }
                Ok(bytes)
            }
            Ok(None) => Ok(fs::read(&local)?),
            Err(error @ RegistryError::Status { status: 404, .. }) => Err(error),
            Err(error) if self.fixed_sources => Err(error),
            Err(error) => match fs::read(&local) {
                Ok(bytes) => {
                    tracing::warn!(registry = %source.name, %path, %error, "using cached registry document");
                    Ok(bytes)
                }
                Err(_) => Err(error),
            },
        }
    }

    pub(super) async fn get(
        &self,
        url: &str,
        etag: Option<&str>,
    ) -> Result<Option<(Vec<u8>, Option<String>)>, RegistryError> {
        let mut request = self.http.get(url);
        if let Some(etag) = etag {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        let mut response = request
            .send()
            .await
            .map_err(|error| RegistryError::http(url, error))?;
        let status = response.status();
        if status == StatusCode::NOT_MODIFIED && etag.is_some() {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(RegistryError::Status {
                url: url.to_owned(),
                status: status.as_u16(),
            });
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_DOCUMENT_BYTES)
        {
            return Err(RegistryError::TooLarge(MAX_DOCUMENT_BYTES));
        }
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| RegistryError::http(url, error))?
        {
            bytes.extend_from_slice(&chunk);
            if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
                return Err(RegistryError::TooLarge(MAX_DOCUMENT_BYTES));
            }
        }
        Ok(Some((bytes, etag)))
    }
}

fn write_cache(path: &Path, bytes: &[u8]) -> Result<(), RegistryError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)?;
    Ok(())
}
