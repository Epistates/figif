//! Configuration file handling using XDG paths.
//!
//! This module is scaffolding for future configuration support.

#![allow(dead_code)]

use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const APP_NAME: &str = "figif";

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,

    #[serde(default)]
    pub presets: Presets,

    #[serde(default)]
    pub encoding: Encoding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    /// Default hash algorithm
    #[serde(default = "default_hasher")]
    pub hasher: String,

    /// Default similarity threshold
    #[serde(default = "default_threshold")]
    pub threshold: u32,

    /// Default output format: table, json, plain
    #[serde(default = "default_output_format")]
    pub output_format: String,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            hasher: default_hasher(),
            threshold: default_threshold(),
            output_format: default_output_format(),
        }
    }
}

fn default_hasher() -> String {
    "dhash".to_string()
}

fn default_threshold() -> u32 {
    5
}

fn default_output_format() -> String {
    "table".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Presets {
    #[serde(default)]
    pub fast: PresetConfig,

    #[serde(default)]
    pub balanced: PresetConfig,

    #[serde(default)]
    pub aggressive: PresetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PresetConfig {
    pub cap_pauses: Option<u32>,
    pub collapse_pauses: Option<u32>,
    pub speed_up_pauses: Option<f64>,
    pub speed_up_all: Option<f64>,
    pub remove_long: Option<u32>,
}

impl Presets {
    /// Get default presets if not configured.
    pub fn with_defaults() -> Self {
        Self {
            fast: PresetConfig {
                cap_pauses: Some(200),
                speed_up_all: Some(1.0),
                ..Default::default()
            },
            balanced: PresetConfig {
                cap_pauses: Some(300),
                speed_up_pauses: Some(1.5),
                ..Default::default()
            },
            aggressive: PresetConfig {
                collapse_pauses: Some(100),
                speed_up_all: Some(1.5),
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encoding {
    /// Default encoder: standard or gifski
    #[serde(default = "default_encoder")]
    pub default_encoder: String,

    /// Default lossy quality (1-100)
    #[serde(default = "default_lossy_quality")]
    pub lossy_quality: u8,
}

impl Default for Encoding {
    fn default() -> Self {
        Self {
            default_encoder: default_encoder(),
            lossy_quality: default_lossy_quality(),
        }
    }
}

fn default_encoder() -> String {
    "standard".to_string()
}

fn default_lossy_quality() -> u8 {
    80
}

impl Config {
    /// Load configuration from the default XDG path or a custom path.
    pub fn load(custom_path: Option<&PathBuf>) -> Result<Self> {
        if let Some(path) = custom_path {
            let content = std::fs::read_to_string(path)
                .wrap_err_with(|| format!("Failed to read config file: {}", path.display()))?;
            let config: Config = toml::from_str(&content)
                .wrap_err_with(|| format!("Failed to parse config file: {}", path.display()))?;
            Ok(config)
        } else {
            // Use confy for XDG-compliant default path
            let config: Config = confy::load(APP_NAME, None).unwrap_or_default();
            Ok(config)
        }
    }

    /// Get the default config file path.
    pub fn default_path() -> Option<PathBuf> {
        confy::get_configuration_file_path(APP_NAME, None).ok()
    }

    /// Get the preset configuration by name.
    pub fn get_preset(&self, name: &str) -> Option<&PresetConfig> {
        match name.to_lowercase().as_str() {
            "fast" => Some(&self.presets.fast),
            "balanced" => Some(&self.presets.balanced),
            "aggressive" => Some(&self.presets.aggressive),
            _ => None,
        }
    }
}

// Add toml dependency for parsing
// Note: confy uses toml internally, but we need it for custom paths
