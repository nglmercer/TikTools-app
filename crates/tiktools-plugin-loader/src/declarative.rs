//! Declarative plugin placeholder runtime.
//!
//! Declarative packages ship interpreted data (HTTP actions, option sources,
//! templates, pages) instead of executable code. The host executes every
//! behavior itself, so starting always succeeds and direct calls are
//! rejected: reaching this instance means a host dispatch path missed its
//! declarative intercept.

use std::path::Path;

use tiktools_plugin_api::{PluginManifest, PluginRuntimeKind};

use crate::{PluginInstance, PluginLoaderError, PluginRuntime};

#[derive(Default)]
pub struct DeclarativePluginRuntime;

struct DeclarativeInstance {
    id: String,
}

impl PluginRuntime for DeclarativePluginRuntime {
    fn kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::Declarative
    }

    fn load(
        &self,
        manifest: &PluginManifest,
        _directory: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginLoaderError> {
        Ok(Box::new(DeclarativeInstance {
            id: manifest.id.clone(),
        }))
    }
}

impl PluginInstance for DeclarativeInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn handle_message(&mut self, _request: &[u8]) -> Result<Vec<u8>, PluginLoaderError> {
        Err(PluginLoaderError::Runtime(format!(
            "plugin `{}` is declarative and is interpreted by the host",
            self.id
        )))
    }

    fn shutdown(&mut self) -> Result<(), PluginLoaderError> {
        Ok(())
    }
}
