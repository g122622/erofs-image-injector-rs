//! Strategy configuration types for mutation strategies
//!
//! This module provides types for configuring mutation strategies used in fuzzing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mutator type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutatorType {
    // Basic mutation strategies
    /// Random bit flipping
    BitFlip,
    /// Random byte replacement
    Random,
    /// Set bytes to zero
    Zero,
    /// Set bytes to 0xFF
    Max,
    /// Arithmetic operations (add/sub)
    Arithmetic,
    /// Use interesting values (edge cases)
    InterestingValues,
    /// Use boundary values
    Boundary,

    // Structure-aware mutators
    /// Superblock-aware mutator
    Superblock,
    /// Inode-aware mutator
    Inode,
    /// Directory entry mutator
    Dirent,
    /// Extended attribute mutator
    Xattr,

    // Targeted mutation
    /// Target specific fields or byte ranges
    Targeted,
}

impl std::fmt::Display for MutatorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutatorType::BitFlip => write!(f, "bitflip"),
            MutatorType::Random => write!(f, "random"),
            MutatorType::Zero => write!(f, "zero"),
            MutatorType::Max => write!(f, "max"),
            MutatorType::Arithmetic => write!(f, "arithmetic"),
            MutatorType::InterestingValues => write!(f, "interesting_values"),
            MutatorType::Boundary => write!(f, "boundary"),
            MutatorType::Superblock => write!(f, "superblock"),
            MutatorType::Inode => write!(f, "inode"),
            MutatorType::Dirent => write!(f, "dirent"),
            MutatorType::Xattr => write!(f, "xattr"),
            MutatorType::Targeted => write!(f, "targeted"),
        }
    }
}

impl std::str::FromStr for MutatorType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bitflip" | "bit_flip" => Ok(Self::BitFlip),
            "random" => Ok(Self::Random),
            "zero" => Ok(Self::Zero),
            "max" => Ok(Self::Max),
            "arithmetic" => Ok(Self::Arithmetic),
            "interesting_values" | "interestingvalues" | "interesting" => Ok(Self::InterestingValues),
            "boundary" => Ok(Self::Boundary),
            "superblock" => Ok(Self::Superblock),
            "inode" => Ok(Self::Inode),
            "dirent" => Ok(Self::Dirent),
            "xattr" => Ok(Self::Xattr),
            "targeted" => Ok(Self::Targeted),
            _ => Err(format!("Unknown mutator type: {}", s)),
        }
    }
}

impl Default for MutatorType {
    fn default() -> Self {
        Self::BitFlip
    }
}

/// Layer type for layered mutation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerType {
    /// Superblock layer
    Superblock,
    /// Inode layer
    Inode,
    /// Directory entry layer
    Dirent,
    /// Data block layer
    DataBlock,
}

impl std::fmt::Display for LayerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayerType::Superblock => write!(f, "superblock"),
            LayerType::Inode => write!(f, "inode"),
            LayerType::Dirent => write!(f, "dirent"),
            LayerType::DataBlock => write!(f, "data_block"),
        }
    }
}

impl std::str::FromStr for LayerType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "superblock" | "sb" => Ok(Self::Superblock),
            "inode" => Ok(Self::Inode),
            "dirent" | "directory" | "directory_entry" => Ok(Self::Dirent),
            "data_block" | "datablock" | "data" => Ok(Self::DataBlock),
            _ => Err(format!("Unknown layer type: {}", s)),
        }
    }
}

/// Mutator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutatorConfig {
    /// Whether this mutator is enabled
    pub enabled: bool,
    /// Weight for this mutator (relative probability)
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// Minimum iterations for this mutator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_iterations: Option<u64>,
    /// Maximum iterations for this mutator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u64>,
    /// Mutator-specific parameters
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub params: HashMap<String, serde_json::Value>,
}

fn default_weight() -> u32 {
    100
}

impl Default for MutatorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            weight: 100,
            min_iterations: None,
            max_iterations: None,
            params: HashMap::new(),
        }
    }
}

/// Target field for targeted mutation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TargetField {
    /// Superblock field
    Superblock {
        /// Field name
        field: String,
    },
    /// Inode field
    Inode {
        /// Field name
        field: String,
        /// Inode index (None for first)
        #[serde(skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    /// Directory entry
    Dirent {
        /// Entry index
        index: usize,
        /// Part to target
        #[serde(default)]
        part: DirentPart,
    },
    /// Absolute byte range
    Range {
        /// Start offset
        start: usize,
        /// Length
        length: usize,
    },
    /// Data block
    DataBlock {
        /// Block number
        block_num: usize,
        /// Offset within block
        #[serde(default)]
        offset_in_block: usize,
        /// Length to mutate
        #[serde(skip_serializing_if = "Option::is_none")]
        length: Option<usize>,
    },
}

/// Part of a directory entry to target
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirentPart {
    /// Entire entry
    #[default]
    All,
    /// Node ID field
    Nid,
    /// Name offset field
    NameOff,
    /// File type field
    FileType,
    /// Name data
    Name,
}

/// Layered mutation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    /// Layer type
    pub layer: LayerType,
    /// Mutators for this layer with their weights
    pub mutators: HashMap<MutatorType, MutatorConfig>,
    /// Target fields for targeted mutation (if applicable)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<TargetField>,
}

/// Adaptive weight adjustment rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveRule {
    /// Event that triggers adjustment
    pub trigger: AdaptiveTrigger,
    /// Mutator to adjust
    pub mutator: MutatorType,
    /// Adjustment percentage (can be negative)
    pub adjustment_percent: i32,
}

/// Trigger for adaptive weight adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdaptiveTrigger {
    /// When a crash is found by this mutator
    CrashFound,
    /// After N iterations without crash
    NoCrashIterations { count: u64 },
    /// When a specific crash type is found
    CrashType { crash_type: String },
}

/// Strategy template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyTemplate {
    /// Template ID (None for new templates)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Template name
    pub name: String,
    /// Template description
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Whether this is a built-in template
    #[serde(default)]
    pub is_builtin: bool,
    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Last modified timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Simple mode: mutator configurations
    pub mutators: HashMap<MutatorType, MutatorConfig>,
    /// Advanced mode: layered configurations
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<LayerConfig>,
    /// Adaptive weight rules
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adaptive_rules: Vec<AdaptiveRule>,
    /// Whether adaptive weights are enabled
    #[serde(default)]
    pub adaptive_enabled: bool,
}

impl Default for StrategyTemplate {
    fn default() -> Self {
        Self {
            id: None,
            name: "New Strategy".to_string(),
            description: String::new(),
            is_builtin: false,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            mutators: HashMap::new(),
            layers: Vec::new(),
            adaptive_rules: Vec::new(),
            adaptive_enabled: false,
        }
    }
}

impl StrategyTemplate {
    /// Create a "Quick Discovery" preset template
    pub fn quick_discovery() -> Self {
        let mut mutators = HashMap::new();
        mutators.insert(MutatorType::BitFlip, MutatorConfig {
            enabled: true,
            weight: 400,
            min_iterations: None,
            max_iterations: None,
            params: {
                let mut p = HashMap::new();
                p.insert("count".to_string(), serde_json::json!(2));
                p
            },
        });
        mutators.insert(MutatorType::Random, MutatorConfig {
            enabled: true,
            weight: 300,
            ..Default::default()
        });
        mutators.insert(MutatorType::Arithmetic, MutatorConfig {
            enabled: true,
            weight: 200,
            min_iterations: None,
            max_iterations: None,
            params: {
                let mut p = HashMap::new();
                p.insert("min_delta".to_string(), serde_json::json!(-16));
                p.insert("max_delta".to_string(), serde_json::json!(16));
                p
            },
        });
        mutators.insert(MutatorType::InterestingValues, MutatorConfig {
            enabled: true,
            weight: 100,
            ..Default::default()
        });

        Self {
            id: Some(-1), // Built-in IDs are negative
            name: "Quick Discovery".to_string(),
            description: "Fast crash discovery using efficient mutation strategies".to_string(),
            is_builtin: true,
            created_at: None,
            updated_at: None,
            mutators,
            layers: Vec::new(),
            adaptive_rules: Vec::new(),
            adaptive_enabled: false,
        }
    }

    /// Create a "Structure Deep" preset template
    pub fn structure_deep() -> Self {
        let mut mutators = HashMap::new();
        mutators.insert(MutatorType::Superblock, MutatorConfig {
            enabled: true,
            weight: 300,
            ..Default::default()
        });
        mutators.insert(MutatorType::Inode, MutatorConfig {
            enabled: true,
            weight: 300,
            ..Default::default()
        });
        mutators.insert(MutatorType::Dirent, MutatorConfig {
            enabled: true,
            weight: 200,
            ..Default::default()
        });
        mutators.insert(MutatorType::Xattr, MutatorConfig {
            enabled: true,
            weight: 100,
            ..Default::default()
        });
        mutators.insert(MutatorType::Targeted, MutatorConfig {
            enabled: true,
            weight: 100,
            ..Default::default()
        });

        Self {
            id: Some(-2),
            name: "Structure Deep".to_string(),
            description: "Deep testing of EROFS structure fields with structure-aware mutators".to_string(),
            is_builtin: true,
            created_at: None,
            updated_at: None,
            mutators,
            layers: Vec::new(),
            adaptive_rules: Vec::new(),
            adaptive_enabled: false,
        }
    }

    /// Create a "Boundary Test" preset template
    pub fn boundary_test() -> Self {
        let mut mutators = HashMap::new();
        mutators.insert(MutatorType::Boundary, MutatorConfig {
            enabled: true,
            weight: 400,
            ..Default::default()
        });
        mutators.insert(MutatorType::InterestingValues, MutatorConfig {
            enabled: true,
            weight: 400,
            min_iterations: None,
            max_iterations: None,
            params: {
                let mut p = HashMap::new();
                p.insert("size".to_string(), serde_json::json!(4));
                p
            },
        });
        mutators.insert(MutatorType::Zero, MutatorConfig {
            enabled: true,
            weight: 100,
            ..Default::default()
        });
        mutators.insert(MutatorType::Max, MutatorConfig {
            enabled: true,
            weight: 100,
            ..Default::default()
        });

        Self {
            id: Some(-3),
            name: "Boundary Test".to_string(),
            description: "Test boundary conditions using edge case values".to_string(),
            is_builtin: true,
            created_at: None,
            updated_at: None,
            mutators,
            layers: Vec::new(),
            adaptive_rules: Vec::new(),
            adaptive_enabled: false,
        }
    }

    /// Create a "Full Coverage" preset template
    pub fn full_coverage() -> Self {
        let mut mutators = HashMap::new();

        // Basic mutators
        mutators.insert(MutatorType::BitFlip, MutatorConfig {
            enabled: true,
            weight: 150,
            params: {
                let mut p = HashMap::new();
                p.insert("count".to_string(), serde_json::json!(1));
                p
            },
            ..Default::default()
        });
        mutators.insert(MutatorType::Random, MutatorConfig {
            enabled: true,
            weight: 100,
            ..Default::default()
        });
        mutators.insert(MutatorType::Arithmetic, MutatorConfig {
            enabled: true,
            weight: 100,
            params: {
                let mut p = HashMap::new();
                p.insert("min_delta".to_string(), serde_json::json!(-32));
                p.insert("max_delta".to_string(), serde_json::json!(32));
                p
            },
            ..Default::default()
        });
        mutators.insert(MutatorType::Zero, MutatorConfig {
            enabled: true,
            weight: 50,
            ..Default::default()
        });
        mutators.insert(MutatorType::Max, MutatorConfig {
            enabled: true,
            weight: 50,
            ..Default::default()
        });
        mutators.insert(MutatorType::InterestingValues, MutatorConfig {
            enabled: true,
            weight: 100,
            ..Default::default()
        });
        mutators.insert(MutatorType::Boundary, MutatorConfig {
            enabled: true,
            weight: 100,
            ..Default::default()
        });

        // Structure-aware mutators
        mutators.insert(MutatorType::Superblock, MutatorConfig {
            enabled: true,
            weight: 150,
            ..Default::default()
        });
        mutators.insert(MutatorType::Inode, MutatorConfig {
            enabled: true,
            weight: 150,
            ..Default::default()
        });
        mutators.insert(MutatorType::Dirent, MutatorConfig {
            enabled: true,
            weight: 100,
            ..Default::default()
        });
        mutators.insert(MutatorType::Xattr, MutatorConfig {
            enabled: true,
            weight: 50,
            ..Default::default()
        });
        mutators.insert(MutatorType::Targeted, MutatorConfig {
            enabled: true,
            weight: 50,
            ..Default::default()
        });

        Self {
            id: Some(-4),
            name: "Full Coverage".to_string(),
            description: "Comprehensive testing using all mutation strategies with balanced weights".to_string(),
            is_builtin: true,
            created_at: None,
            updated_at: None,
            mutators,
            layers: Vec::new(),
            adaptive_rules: Vec::new(),
            adaptive_enabled: false,
        }
    }

    /// Get all built-in templates
    pub fn builtins() -> Vec<Self> {
        vec![
            Self::quick_discovery(),
            Self::structure_deep(),
            Self::boundary_test(),
            Self::full_coverage(),
        ]
    }

    /// Validate the template configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Template name cannot be empty".to_string());
        }

        let enabled_count = self.mutators.values().filter(|m| m.enabled).count();
        if enabled_count == 0 {
            return Err("At least one mutator must be enabled".to_string());
        }

        // Validate weights
        for (mutator_type, config) in &self.mutators {
            if config.enabled && config.weight == 0 {
                return Err(format!(
                    "Weight for enabled mutator '{}' cannot be zero",
                    mutator_type
                ));
            }
            if config.weight > 1000 {
                return Err(format!(
                    "Weight for mutator '{}' exceeds maximum (1000)",
                    mutator_type
                ));
            }
        }

        // Validate layers
        for layer in &self.layers {
            let layer_enabled = layer.mutators.values().filter(|m| m.enabled).count();
            if layer_enabled == 0 && !layer.targets.is_empty() {
                return Err(format!(
                    "Layer '{}' has targets but no enabled mutators",
                    layer.layer
                ));
            }
        }

        Ok(())
    }

    /// Calculate the normalized weights for enabled mutators
    pub fn normalized_weights(&self) -> HashMap<MutatorType, f64> {
        let total: u64 = self.mutators
            .values()
            .filter(|m| m.enabled)
            .map(|m| m.weight as u64)
            .sum();

        if total == 0 {
            return HashMap::new();
        }

        self.mutators
            .iter()
            .filter(|(_, m)| m.enabled)
            .map(|(t, m)| (*t, m.weight as f64 / total as f64))
            .collect()
    }
}

/// Create strategy template request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStrategyRequest {
    /// Template name
    pub name: String,
    /// Template description
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Mutator configurations
    #[serde(default)]
    pub mutators: HashMap<MutatorType, MutatorConfig>,
    /// Layered configurations
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<LayerConfig>,
    /// Adaptive weight rules
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adaptive_rules: Vec<AdaptiveRule>,
    /// Whether adaptive weights are enabled
    #[serde(default)]
    pub adaptive_enabled: bool,
}

/// Update strategy template request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateStrategyRequest {
    /// Template name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Template description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Mutator configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutators: Option<HashMap<MutatorType, MutatorConfig>>,
    /// Layered configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<LayerConfig>>,
    /// Adaptive weight rules
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptive_rules: Option<Vec<AdaptiveRule>>,
    /// Whether adaptive weights are enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptive_enabled: Option<bool>,
}

/// Mutator runtime statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutatorStats {
    /// Mutator type
    pub mutator: MutatorType,
    /// Total executions
    pub executions: u64,
    /// Crashes found
    pub crashes: u64,
    /// Current weight (may change with adaptive)
    pub current_weight: u32,
    /// Original weight
    pub original_weight: u32,
}

impl MutatorStats {
    /// Calculate crash rate
    pub fn crash_rate(&self) -> f64 {
        if self.executions == 0 {
            return 0.0;
        }
        self.crashes as f64 / self.executions as f64
    }
}

/// Strategy runtime statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyStats {
    /// Task ID
    pub task_id: i64,
    /// Strategy template ID
    pub strategy_id: Option<i64>,
    /// Strategy name
    pub strategy_name: String,
    /// Per-mutator statistics
    pub mutators: Vec<MutatorStats>,
    /// Total iterations
    pub total_iterations: u64,
    /// Total crashes
    pub total_crashes: u64,
    /// Whether adaptive weights are active
    pub adaptive_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_templates() {
        let templates = StrategyTemplate::builtins();
        assert_eq!(templates.len(), 4);

        for template in &templates {
            assert!(template.validate().is_ok());
        }
    }

    #[test]
    fn test_normalized_weights() {
        let template = StrategyTemplate::quick_discovery();
        let weights = template.normalized_weights();

        let total: f64 = weights.values().sum();
        assert!((total - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_validate_empty_name() {
        let mut template = StrategyTemplate::quick_discovery();
        template.name = "".to_string();
        assert!(template.validate().is_err());
    }

    #[test]
    fn test_validate_no_enabled_mutators() {
        let mut template = StrategyTemplate::quick_discovery();
        for config in template.mutators.values_mut() {
            config.enabled = false;
        }
        assert!(template.validate().is_err());
    }

    #[test]
    fn test_mutator_type_from_str() {
        assert_eq!(MutatorType::from_str("bitflip").unwrap(), MutatorType::BitFlip);
        assert_eq!(MutatorType::from_str("random").unwrap(), MutatorType::Random);
        assert_eq!(MutatorType::from_str("superblock").unwrap(), MutatorType::Superblock);
        assert!(MutatorType::from_str("unknown").is_err());
    }

    #[test]
    fn test_layer_type_from_str() {
        assert_eq!(LayerType::from_str("superblock").unwrap(), LayerType::Superblock);
        assert_eq!(LayerType::from_str("inode").unwrap(), LayerType::Inode);
        assert_eq!(LayerType::from_str("dirent").unwrap(), LayerType::Dirent);
        assert_eq!(LayerType::from_str("data_block").unwrap(), LayerType::DataBlock);
    }
}
