use super::*;

impl RegistryClient {
    pub async fn catalog_page(
        &self,
        registry: &str,
        page: u32,
    ) -> Result<CatalogPage, RegistryError> {
        let _operation = self.operation_guard().await;
        let source = self.source(registry)?;
        let (source, _) = self.ensure_loaded(&source).await?;
        let path = format!("catalog/page-{}.json", page.max(1));
        let bytes = self.document(&source, &path, KeyRole::Index).await?;
        serde_json::from_slice(&bytes).map_err(|error| RegistryError::document(&path, error))
    }

    pub async fn search(&self, query: &SearchQuery) -> SearchResults {
        let _operation = self.operation_guard().await;
        let words: Vec<String> = query
            .query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let sources: Vec<RegistrySource> = match &query.registry {
            Some(name) => self.source(name).into_iter().collect(),
            None => self.config().ordered().into_iter().cloned().collect(),
        };
        let mut seen = BTreeSet::new();
        let mut entries = Vec::new();
        let mut errors = Vec::new();
        for source in sources {
            match self.registry_entries(&source, query, &words).await {
                Ok(found) => {
                    for entry in found {
                        if seen.insert(entry.id.clone()) {
                            entries.push(SearchEntry {
                                registry: source.name.clone(),
                                entry,
                            });
                        }
                    }
                }
                Err(error) => errors.push(format!("{}: {error}", source.name)),
            }
        }
        let total = entries.len();
        let pages = total.div_ceil(SEARCH_PAGE_SIZE).max(1);
        let page = (query.page.max(1) as usize).min(pages);
        let entries = entries
            .into_iter()
            .skip((page - 1) * SEARCH_PAGE_SIZE)
            .take(SEARCH_PAGE_SIZE)
            .collect();
        SearchResults {
            entries,
            total: total as u32,
            page: page as u32,
            pages: pages as u32,
            errors,
        }
    }

    async fn registry_entries(
        &self,
        source: &RegistrySource,
        query: &SearchQuery,
        words: &[String],
    ) -> Result<Vec<CatalogEntry>, RegistryError> {
        let (source, config) = self.ensure_loaded(source).await?;
        if let Some(api) = config.api.as_deref().filter(|_| !self.fixed_sources) {
            let mut url = url::Url::parse(&join_url(api, "search")?)
                .map_err(|_| RegistryError::InsecureUrl(api.to_owned()))?;
            url.query_pairs_mut()
                .append_pair("q", &query.query)
                .append_pair("category", query.category.as_deref().unwrap_or_default());
            if let Some((bytes, _)) = self.get(url.as_str(), None).await? {
                let page: CatalogPage = serde_json::from_slice(&bytes)
                    .map_err(|error| RegistryError::document("search", error))?;
                return Ok(page.entries);
            }
        }
        let mut entries = Vec::new();
        let pages = config.catalog_pages.clamp(1, MAX_CATALOG_PAGES);
        for page in 1..=pages {
            let path = format!("catalog/page-{}.json", page.max(1));
            let bytes = self.document(&source, &path, KeyRole::Index).await?;
            let catalog: CatalogPage = serde_json::from_slice(&bytes)
                .map_err(|error| RegistryError::document(&path, error))?;
            entries.extend(
                catalog
                    .entries
                    .into_iter()
                    .filter(|entry| entry.matches(words, query.category.as_deref())),
            );
            if page >= catalog.pages {
                break;
            }
        }
        Ok(entries)
    }
}
