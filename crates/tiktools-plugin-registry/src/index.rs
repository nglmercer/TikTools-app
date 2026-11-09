use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tiktools_plugin_api::{manifest::HOST_ENGINE, PluginManifest};
use tiktools_plugin_package::{KeysDocument, PackageArchive, Sha256Digest, SigningKey, TrustedKey};

use crate::{
    client::now, model::shard_path, Advisory, Artifact, CatalogEntry, CatalogPage, PackageDocument,
    RegistryConfigDocument, RegistryError, Release, RootDocument, ANY_TARGET,
    REGISTRY_PROTOCOL_VERSION,
};

const CATALOG_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedRelease {
    pub id: String,
    pub version: String,
    pub target: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
}

pub struct IndexWriter {
    root: PathBuf,
    name: String,
    index_key: Option<SigningKey>,
}

impl IndexWriter {
    pub fn open(
        root: impl Into<PathBuf>,
        name: impl Into<String>,
        index_key: Option<SigningKey>,
    ) -> Self {
        Self {
            root: root.into(),
            name: name.into(),
            index_key,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn init(
        &self,
        root_key: &SigningKey,
        delegations: Vec<TrustedKey>,
        description: Option<String>,
        api: Option<String>,
    ) -> Result<(), RegistryError> {
        fs::create_dir_all(&self.root)?;
        let root = RootDocument {
            version: 1,
            keys: vec![root_key.public_key()],
        };
        self.write_plain("root.json", &root)?;
        let keys = KeysDocument::new(delegations);
        let keys_bytes = keys.to_bytes();
        fs::write(self.root.join("keys.json"), &keys_bytes)?;
        fs::write(
            self.root.join("keys.json.sig"),
            root_key.sign_detached(&keys_bytes).to_bytes(),
        )?;
        let mut config = self.load_config()?;
        config.description = description.or(config.description);
        config.api = api.or(config.api);
        self.write_config(config)
    }

    pub fn add(
        &self,
        package: &Path,
        channel: &str,
        artifact_base_url: Option<&str>,
    ) -> Result<IndexedRelease, RegistryError> {
        let archive = PackageArchive::open(package)?;
        let inspection = archive.inspect()?;
        let manifest = &inspection.manifest;
        let version = manifest.semver().ok_or_else(|| {
            RegistryError::document(&manifest.id, "registry packages need a semver version")
        })?;
        let digest = archive.digest()?;
        let file_name = format!(
            "{}-{}{}.plugin",
            manifest.id,
            version,
            manifest
                .targets
                .first()
                .map(|target| format!("-{target}"))
                .unwrap_or_default()
        );
        let url = match artifact_base_url {
            Some(base) => format!("{}/{file_name}", base.trim_end_matches('/')),
            None => {
                let relative = format!("packages/{}/{file_name}", manifest.id);
                let destination = self.root.join(&relative);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(archive.path(), &destination)?;
                relative
            }
        };
        let target = manifest
            .targets
            .first()
            .cloned()
            .unwrap_or_else(|| ANY_TARGET.to_owned());
        let key_id = inspection
            .signature
            .as_ref()
            .map(|signature| signature.key_id.to_string());
        let artifact = Artifact {
            target: target.clone(),
            url: url.clone(),
            size: archive.size(),
            sha256: digest.clone(),
            key_id: key_id.clone(),
        };

        let path = shard_path(&manifest.id);
        let mut document = self
            .read_json::<PackageDocument>(&path)?
            .unwrap_or_else(|| empty_document(manifest));
        let release = release_from(manifest, channel, inspection.changelog.clone(), artifact);
        match document
            .releases
            .iter_mut()
            .find(|existing| existing.semver().as_ref() == Some(&version))
        {
            Some(existing) => {
                if existing.runtime != release.runtime
                    || existing.permissions != release.permissions
                    || existing.capabilities != release.capabilities
                {
                    return Err(RegistryError::document(
                        &path,
                        format!(
                            "{} {version} was published with different metadata",
                            manifest.id
                        ),
                    ));
                }
                existing
                    .artifacts
                    .retain(|artifact| artifact.target != target);
                existing.artifacts.extend(release.artifacts);
            }
            None => document.releases.push(release),
        }
        let is_latest = document
            .releases
            .iter()
            .filter_map(Release::semver)
            .all(|existing| existing <= version);
        if is_latest {
            apply_metadata(&mut document, manifest);
        }
        self.finish_document(&path, document)?;
        self.rebuild()?;
        Ok(IndexedRelease {
            id: manifest.id.clone(),
            version: version.to_string(),
            target,
            url,
            sha256: digest.to_string(),
            size: archive.size(),
            key_id,
        })
    }

    pub fn yank(&self, id: &str, version: &str, yanked: bool) -> Result<(), RegistryError> {
        self.update_release(id, version, |release| release.yanked = yanked)
    }

    pub fn advise(
        &self,
        id: &str,
        version: &str,
        advisory: Option<Advisory>,
    ) -> Result<(), RegistryError> {
        self.update_release(id, version, |release| release.advisory = advisory)
    }

    pub fn revoke_digest(&self, digest: &Sha256Digest) -> Result<(), RegistryError> {
        let mut config = self.load_config()?;
        if !config
            .revoked_digests
            .iter()
            .any(|value| value == digest.as_hex())
        {
            config.revoked_digests.push(digest.to_string());
        }
        self.write_config(config)
    }

    pub fn rebuild(&self) -> Result<(), RegistryError> {
        let mut entries = Vec::new();
        let index = self.root.join("index");
        if index.is_dir() {
            let mut pending = vec![index];
            while let Some(directory) = pending.pop() {
                for entry in fs::read_dir(&directory)? {
                    let path = entry?.path();
                    if path.is_dir() {
                        pending.push(path);
                    } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
                        let document: PackageDocument = serde_json::from_slice(&fs::read(&path)?)
                            .map_err(|error| {
                            RegistryError::document(&path.display().to_string(), error)
                        })?;
                        if let Some(entry) = catalog_entry(&document) {
                            entries.push(entry);
                        }
                    }
                }
            }
        }
        entries.sort_by(|left, right| left.id.cmp(&right.id));
        let total = entries.len();
        let pages = total.div_ceil(CATALOG_PAGE_SIZE).max(1);
        let catalog = self.root.join("catalog");
        if catalog.is_dir() {
            fs::remove_dir_all(&catalog)?;
        }
        for page in 1..=pages {
            let chunk = entries
                .iter()
                .skip((page - 1) * CATALOG_PAGE_SIZE)
                .take(CATALOG_PAGE_SIZE)
                .cloned()
                .collect();
            self.write_signed(
                &format!("catalog/page-{page}.json"),
                &CatalogPage {
                    page: page as u32,
                    pages: pages as u32,
                    total: total as u32,
                    entries: chunk,
                },
            )?;
        }
        let mut config = self.load_config()?;
        config.catalog_pages = pages as u32;
        self.write_config(config)
    }

    fn update_release(
        &self,
        id: &str,
        version: &str,
        change: impl FnOnce(&mut Release),
    ) -> Result<(), RegistryError> {
        if !tiktools_plugin_api::manifest::is_valid_plugin_id(id) {
            return Err(RegistryError::document(id, "invalid plugin ID"));
        }
        let path = shard_path(id);
        let mut document = self
            .read_json::<PackageDocument>(&path)?
            .ok_or_else(|| RegistryError::NotFound(id.to_owned()))?;
        let wanted = tiktools_plugin_api::manifest::parse_version(version)
            .ok_or_else(|| RegistryError::document(&path, format!("invalid version {version}")))?;
        let release = document
            .releases
            .iter_mut()
            .find(|release| release.semver().as_ref() == Some(&wanted))
            .ok_or_else(|| RegistryError::NotFound(format!("{id}@{version}")))?;
        change(release);
        self.finish_document(&path, document)?;
        self.rebuild()
    }

    fn finish_document(
        &self,
        path: &str,
        mut document: PackageDocument,
    ) -> Result<(), RegistryError> {
        document.releases.sort_by_key(Release::semver);
        document.dist_tags = dist_tags(&document.releases);
        self.write_signed(path, &document)
    }

    fn load_config(&self) -> Result<RegistryConfigDocument, RegistryError> {
        Ok(self
            .read_json::<RegistryConfigDocument>("config.json")?
            .unwrap_or_else(|| RegistryConfigDocument {
                protocol: REGISTRY_PROTOCOL_VERSION,
                name: self.name.clone(),
                description: None,
                api: None,
                mirrors: Vec::new(),
                catalog_pages: 0,
                revoked_digests: Vec::new(),
                generated_at: 0,
                expires: None,
            }))
    }

    fn write_config(&self, mut config: RegistryConfigDocument) -> Result<(), RegistryError> {
        config.name = self.name.clone();
        config.protocol = REGISTRY_PROTOCOL_VERSION;
        config.generated_at = now();
        self.write_signed("config.json", &config)
    }

    fn read_json<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>, RegistryError> {
        match fs::read(self.root.join(path)) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| RegistryError::document(path, error)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn write_plain<T: Serialize>(&self, path: &str, value: &T) -> Result<Vec<u8>, RegistryError> {
        let destination = self.root.join(path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut bytes = serde_json::to_vec_pretty(value)
            .map_err(|error| RegistryError::document(path, error))?;
        bytes.push(b'\n');
        fs::write(&destination, &bytes)?;
        Ok(bytes)
    }

    fn write_signed<T: Serialize>(&self, path: &str, value: &T) -> Result<(), RegistryError> {
        let bytes = self.write_plain(path, value)?;
        let signature_path = self.root.join(format!("{path}.sig"));
        match &self.index_key {
            Some(key) => fs::write(signature_path, key.sign_detached(&bytes).to_bytes())?,
            None => {
                let _ = fs::remove_file(signature_path);
            }
        }
        Ok(())
    }
}

fn empty_document(manifest: &PluginManifest) -> PackageDocument {
    let mut document = PackageDocument {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        publisher: None,
        description: None,
        icon: None,
        categories: Vec::new(),
        tags: Vec::new(),
        homepage: None,
        repository: None,
        license: None,
        screenshots: Vec::new(),
        dist_tags: BTreeMap::new(),
        releases: Vec::new(),
    };
    apply_metadata(&mut document, manifest);
    document
}

fn apply_metadata(document: &mut PackageDocument, manifest: &PluginManifest) {
    document.name = manifest.name.clone();
    document.publisher = manifest.metadata.publisher.clone();
    document.description = manifest.description.clone();
    document.icon = manifest.icon.clone();
    document.categories = manifest.metadata.categories.clone();
    document.tags = manifest.tags.clone();
    document.homepage = manifest.metadata.homepage.clone();
    document.repository = manifest.metadata.repository.clone();
    document.license = manifest.metadata.license.clone();
}

fn release_from(
    manifest: &PluginManifest,
    channel: &str,
    changelog: Option<String>,
    artifact: Artifact,
) -> Release {
    let mut permissions = manifest.permissions.clone();
    permissions.sort();
    let mut capabilities = manifest.capabilities.clone();
    capabilities.sort();
    Release {
        version: manifest
            .semver()
            .map(|version| version.to_string())
            .unwrap_or_else(|| manifest.version.clone()),
        channel: channel.to_owned(),
        published_at: now(),
        yanked: false,
        advisory: None,
        runtime: manifest.runtime.to_string(),
        engines: manifest
            .metadata
            .engine
            .iter()
            .map(|range| (HOST_ENGINE.to_owned(), range.clone()))
            .collect(),
        schema_version: manifest.schema_version,
        protocol_version: manifest.protocol_version,
        abi_version: manifest.abi_version,
        permissions,
        capabilities,
        dependencies: manifest.metadata.dependencies.clone(),
        changelog,
        artifacts: vec![artifact],
    }
}

fn dist_tags(releases: &[Release]) -> BTreeMap<String, String> {
    let mut tags: BTreeMap<String, (semver::Version, String)> = BTreeMap::new();
    for release in releases.iter().filter(|release| !release.yanked) {
        let Some(version) = release.semver() else {
            continue;
        };
        let replace = tags
            .get(&release.channel)
            .is_none_or(|(current, _)| version > *current);
        if replace {
            tags.insert(release.channel.clone(), (version, release.version.clone()));
        }
    }
    tags.into_iter()
        .map(|(channel, (_, version))| (channel, version))
        .collect()
}

fn catalog_entry(document: &PackageDocument) -> Option<CatalogEntry> {
    let latest = document
        .releases
        .iter()
        .filter(|release| !release.yanked)
        .max_by(|left, right| left.semver().cmp(&right.semver()))?;
    Some(CatalogEntry {
        id: document.id.clone(),
        name: document.name.clone(),
        publisher: document.publisher.clone(),
        description: document.description.clone(),
        icon: document.icon.clone(),
        categories: document.categories.clone(),
        tags: document.tags.clone(),
        latest: latest.version.clone(),
        runtime: latest.runtime.clone(),
        updated_at: latest.published_at,
    })
}
