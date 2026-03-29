//! Strategy template storage module
//!
//! Handles loading, saving, and managing strategy templates as TOML files.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::strategy_types::{StrategyTemplate, CreateStrategyRequest, UpdateStrategyRequest};

/// Strategy storage error type
#[derive(Debug, thiserror::Error)]
pub enum StrategyError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// TOML serialization error
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// TOML deserialization error
    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    /// Template not found
    #[error("Strategy template not found: {0}")]
    NotFound(i64),

    /// Invalid template
    #[error("Invalid strategy template: {0}")]
    Invalid(String),

    /// Cannot modify built-in template
    #[error("Cannot modify built-in template: {0}")]
    CannotModifyBuiltin(i64),

    /// ID generation error
    #[error("Failed to generate template ID")]
    IdGeneration,
}

/// Strategy template storage manager
#[derive(Debug, Clone)]
pub struct StrategyStorage {
    /// Base directory for strategy templates
    base_dir: PathBuf,
    /// In-memory cache of templates
    cache: Arc<RwLock<HashMap<i64, StrategyTemplate>>>,
    /// Next available ID for custom templates
    next_id: Arc<RwLock<i64>>,
}

impl StrategyStorage {
    /// Create a new strategy storage
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self, StrategyError> {
        let base_dir = base_dir.as_ref().to_path_buf();

        // Ensure directory exists
        fs::create_dir_all(&base_dir)?;

        info!("Strategy storage initialized at {:?}", base_dir);

        let storage = Self {
            base_dir,
            cache: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
        };

        Ok(storage)
    }

    /// Create storage in the default user config directory
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("erofs-fuzzer")
            .join("strategies")
    }

    /// Create storage with default path
    pub fn with_default_path() -> Result<Self, StrategyError> {
        Self::new(Self::default_path())
    }

    /// Initialize storage by loading all templates
    pub async fn initialize(&self) -> Result<(), StrategyError> {
        let mut cache = self.cache.write().await;
        let mut next_id = self.next_id.write().await;

        // Load built-in templates first
        for builtin in StrategyTemplate::builtins() {
            if let Some(id) = builtin.id {
                cache.insert(id, builtin);
            }
        }

        // Load custom templates from files
        self.load_custom_templates(&mut cache, &mut next_id)?;

        info!("Loaded {} strategy templates", cache.len());
        Ok(())
    }

    /// Load custom templates from the storage directory
    fn load_custom_templates(
        &self,
        cache: &mut HashMap<i64, StrategyTemplate>,
        next_id: &mut i64,
    ) -> Result<(), StrategyError> {
        let entries = match fs::read_dir(&self.base_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "toml") {
                match self.load_template_file(&path) {
                    Ok(mut template) => {
                        // Assign a new ID if not present
                        if template.id.is_none() {
                            template.id = Some(*next_id);
                            *next_id += 1;
                        } else if let Some(id) = template.id {
                            if id >= *next_id {
                                *next_id = id + 1;
                            }
                        }

                        if let Some(id) = template.id {
                            cache.insert(id, template);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to load template from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a template from a TOML file
    fn load_template_file(&self, path: &Path) -> Result<StrategyTemplate, StrategyError> {
        let content = fs::read_to_string(path)?;
        let template: StrategyTemplate = toml::from_str(&content)?;
        template.validate().map_err(StrategyError::Invalid)?;
        Ok(template)
    }

    /// Save a template to a TOML file
    fn save_template_file(&self, template: &StrategyTemplate) -> Result<(), StrategyError> {
        let id = template.id.ok_or(StrategyError::IdGeneration)?;
        let filename = self.sanitize_filename(&template.name, id);
        let path = self.base_dir.join(&filename);

        let content = toml::to_string_pretty(template)?;
        fs::write(&path, content)?;

        debug!("Saved strategy template to {:?}", path);
        Ok(())
    }

    /// Delete a template file
    fn delete_template_file(&self, id: i64, name: &str) -> Result<(), StrategyError> {
        let filename = self.sanitize_filename(name, id);
        let path = self.base_dir.join(&filename);

        if path.exists() {
            fs::remove_file(&path)?;
            debug!("Deleted strategy template file {:?}", path);
        }

        Ok(())
    }

    /// Sanitize a filename for safe filesystem use
    fn sanitize_filename(&self, name: &str, id: i64) -> String {
        let safe_name: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("{}_{:04}.toml", safe_name, id)
    }

    /// List all templates
    pub async fn list(&self) -> Vec<StrategyTemplate> {
        let cache = self.cache.read().await;
        let mut templates: Vec<_> = cache.values().cloned().collect();
        templates.sort_by(|a, b| {
            // Built-ins first, then by name
            match (a.is_builtin, b.is_builtin) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        templates
    }

    /// Get a template by ID
    pub async fn get(&self, id: i64) -> Option<StrategyTemplate> {
        let cache = self.cache.read().await;
        cache.get(&id).cloned()
    }

    /// Create a new template
    pub async fn create(&self, request: CreateStrategyRequest) -> Result<StrategyTemplate, StrategyError> {
        let mut next_id = self.next_id.write().await;
        let id = *next_id;
        *next_id += 1;

        let now = chrono::Utc::now();
        let template = StrategyTemplate {
            id: Some(id),
            name: request.name,
            description: request.description,
            is_builtin: false,
            created_at: Some(now),
            updated_at: Some(now),
            mutators: request.mutators,
            layers: request.layers,
            adaptive_rules: request.adaptive_rules,
            adaptive_enabled: request.adaptive_enabled,
        };

        // Validate
        template.validate().map_err(StrategyError::Invalid)?;

        // Save to file
        self.save_template_file(&template)?;

        // Add to cache
        let mut cache = self.cache.write().await;
        cache.insert(id, template.clone());

        info!("Created strategy template '{}' with id {}", template.name, id);
        Ok(template)
    }

    /// Update a template
    pub async fn update(&self, id: i64, request: UpdateStrategyRequest) -> Result<StrategyTemplate, StrategyError> {
        let mut cache = self.cache.write().await;

        // Check if template exists
        let template = cache.get(&id).ok_or(StrategyError::NotFound(id))?;

        // Cannot modify built-in templates
        if template.is_builtin {
            return Err(StrategyError::CannotModifyBuiltin(id));
        }

        // Build updated template
        let now = chrono::Utc::now();
        let mut updated = template.clone();
        updated.name = request.name.unwrap_or(template.name.clone());
        updated.description = request.description.unwrap_or_else(|| template.description.clone());
        updated.mutators = request.mutators.unwrap_or_else(|| template.mutators.clone());
        updated.layers = request.layers.unwrap_or_else(|| template.layers.clone());
        updated.adaptive_rules = request.adaptive_rules.unwrap_or_else(|| template.adaptive_rules.clone());
        updated.adaptive_enabled = request.adaptive_enabled.unwrap_or(template.adaptive_enabled);
        updated.updated_at = Some(now);

        // Validate
        updated.validate().map_err(StrategyError::Invalid)?;

        // Save to file
        self.save_template_file(&updated)?;

        // Update cache
        cache.insert(id, updated.clone());

        info!("Updated strategy template '{}' (id {})", updated.name, id);
        Ok(updated)
    }

    /// Delete a template
    pub async fn delete(&self, id: i64) -> Result<(), StrategyError> {
        let mut cache = self.cache.write().await;

        // Check if template exists
        let template = cache.get(&id).ok_or(StrategyError::NotFound(id))?;

        // Cannot delete built-in templates
        if template.is_builtin {
            return Err(StrategyError::CannotModifyBuiltin(id));
        }

        // Delete file
        self.delete_template_file(id, &template.name)?;

        // Remove from cache
        cache.remove(&id);

        info!("Deleted strategy template id {}", id);
        Ok(())
    }

    /// Duplicate a template
    pub async fn duplicate(&self, id: i64, new_name: Option<String>) -> Result<StrategyTemplate, StrategyError> {
        let cache = self.cache.read().await;
        let template = cache.get(&id).ok_or(StrategyError::NotFound(id))?;

        let name = new_name.unwrap_or_else(|| format!("{} (Copy)", template.name));
        let request = CreateStrategyRequest {
            name,
            description: template.description.clone(),
            mutators: template.mutators.clone(),
            layers: template.layers.clone(),
            adaptive_rules: template.adaptive_rules.clone(),
            adaptive_enabled: template.adaptive_enabled,
        };

        drop(cache);
        self.create(request).await
    }

    /// Export a template as TOML string
    pub async fn export(&self, id: i64) -> Result<String, StrategyError> {
        let cache = self.cache.read().await;
        let template = cache.get(&id).ok_or(StrategyError::NotFound(id))?;

        // Create export version without id and timestamps
        let export_template = StrategyTemplate {
            id: None,
            name: template.name.clone(),
            description: template.description.clone(),
            is_builtin: false,
            created_at: None,
            updated_at: None,
            mutators: template.mutators.clone(),
            layers: template.layers.clone(),
            adaptive_rules: template.adaptive_rules.clone(),
            adaptive_enabled: template.adaptive_enabled,
        };

        Ok(toml::to_string_pretty(&export_template)?)
    }

    /// Import a template from TOML string
    pub async fn import(&self, toml_content: &str) -> Result<StrategyTemplate, StrategyError> {
        let mut template: StrategyTemplate = toml::from_str(toml_content)?;
        template.validate().map_err(StrategyError::Invalid)?;

        // Reset ID and timestamps
        template.id = None;
        template.is_builtin = false;
        template.created_at = None;
        template.updated_at = None;

        let request = CreateStrategyRequest {
            name: template.name,
            description: template.description,
            mutators: template.mutators,
            layers: template.layers,
            adaptive_rules: template.adaptive_rules,
            adaptive_enabled: template.adaptive_enabled,
        };

        self.create(request).await
    }

    /// Import a template from a file
    pub async fn import_file<P: AsRef<Path>>(&self, path: P) -> Result<StrategyTemplate, StrategyError> {
        let content = fs::read_to_string(path.as_ref())?;
        self.import(&content).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_create_and_get() {
        let dir = tempdir().unwrap();
        let storage = StrategyStorage::new(dir.path()).unwrap();
        storage.initialize().await.unwrap();

        let request = CreateStrategyRequest {
            name: "Test Strategy".to_string(),
            description: "A test strategy".to_string(),
            mutators: Default::default(),
            layers: vec![],
            adaptive_rules: vec![],
            adaptive_enabled: false,
        };

        let template = storage.create(request).await.unwrap();
        assert!(template.id.is_some());

        let retrieved = storage.get(template.id.unwrap()).await.unwrap();
        assert_eq!(retrieved.name, "Test Strategy");
    }

    #[tokio::test]
    async fn test_storage_list_includes_builtins() {
        let dir = tempdir().unwrap();
        let storage = StrategyStorage::new(dir.path()).unwrap();
        storage.initialize().await.unwrap();

        let templates = storage.list().await;
        assert!(templates.len() >= 4); // At least 4 built-ins
    }

    #[tokio::test]
    async fn test_storage_cannot_delete_builtin() {
        let dir = tempdir().unwrap();
        let storage = StrategyStorage::new(dir.path()).unwrap();
        storage.initialize().await.unwrap();

        let result = storage.delete(-1).await;
        assert!(matches!(result, Err(StrategyError::CannotModifyBuiltin(_))));
    }

    #[tokio::test]
    async fn test_storage_export_import() {
        let dir = tempdir().unwrap();
        let storage = StrategyStorage::new(dir.path()).unwrap();
        storage.initialize().await.unwrap();

        let exported = storage.export(-1).await.unwrap();
        let imported = storage.import(&exported).await.unwrap();

        assert!(imported.id.is_some());
        assert!(imported.id.unwrap() > 0);
    }
}
