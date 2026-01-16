//! NevoFlux Kernel - Browser integration via UniFFI
//!
//! Provides privacy filtering and data processing for browser.nevoflux.* API

use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::RwLock;

// ========== Error Types ==========

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum KernelError {
    #[error("Filter error: {message}")]
    FilterError { message: String },

    #[error("Config error: {message}")]
    ConfigError { message: String },
}

// ========== Privacy Configuration ==========

#[derive(Debug, Clone, uniffi::Record)]
pub struct PrivacyConfig {
    pub enabled: bool,
    pub filter_phone: bool,
    pub filter_id_card: bool,
    pub filter_email: bool,
    pub filter_bank_card: bool,
    pub mode: String,  // "redact" | "partial" | "remove"
    pub scope: String, // "all" | "external_only"
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            filter_phone: true,
            filter_id_card: true,
            filter_email: true,
            filter_bank_card: true,
            mode: "redact".to_string(),
            scope: "external_only".to_string(),
        }
    }
}

// ========== Filter Result ==========

#[derive(Debug, Clone, uniffi::Record)]
pub struct FilterResult {
    pub text: String,
    pub filtered_count: u32,
    pub filtered_items: Vec<FilteredItem>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FilteredItem {
    pub item_type: String,
    pub position: u32,
}

// ========== Global State ==========

static PRIVACY_CONFIG: Lazy<RwLock<PrivacyConfig>> =
    Lazy::new(|| RwLock::new(PrivacyConfig::default()));

// ========== UniFFI Exported Functions ==========

/// Update the global privacy configuration
#[uniffi::export]
pub fn update_privacy_config(config: PrivacyConfig) {
    let mut cfg = PRIVACY_CONFIG.write().unwrap();
    *cfg = config;
}

/// Get the current privacy configuration
#[uniffi::export]
pub fn get_privacy_config() -> PrivacyConfig {
    PRIVACY_CONFIG.read().unwrap().clone()
}

/// Filter sensitive information from text using current config
#[uniffi::export]
pub fn filter_with_config(text: String) -> FilterResult {
    let config = PRIVACY_CONFIG.read().unwrap().clone();

    if !config.enabled {
        return FilterResult {
            text,
            filtered_count: 0,
            filtered_items: vec![],
        };
    }

    filter_sensitive(
        text,
        config.filter_phone,
        config.filter_id_card,
        config.filter_email,
        config.filter_bank_card,
    )
}

/// Filter sensitive information from text with explicit options
#[uniffi::export]
pub fn filter_sensitive(
    text: String,
    phone: bool,
    id_card: bool,
    email: bool,
    bank_card: bool,
) -> FilterResult {
    let mut result_text = text.clone();
    let mut filtered_items = Vec::new();

    // Phone number filter (Chinese mobile)
    if phone {
        let phone_regex = Regex::new(r"1[3-9]\d{9}").unwrap();
        for mat in phone_regex.find_iter(&text) {
            filtered_items.push(FilteredItem {
                item_type: "phone".to_string(),
                position: mat.start() as u32,
            });
        }
        result_text = phone_regex
            .replace_all(&result_text, "[手机号已隐藏]")
            .to_string();
    }

    // ID card filter (Chinese)
    if id_card {
        let id_regex = Regex::new(r"\d{17}[\dXx]").unwrap();
        for mat in id_regex.find_iter(&text) {
            filtered_items.push(FilteredItem {
                item_type: "id_card".to_string(),
                position: mat.start() as u32,
            });
        }
        result_text = id_regex
            .replace_all(&result_text, "[身份证已隐藏]")
            .to_string();
    }

    // Email filter
    if email {
        let email_regex = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
        for mat in email_regex.find_iter(&text) {
            filtered_items.push(FilteredItem {
                item_type: "email".to_string(),
                position: mat.start() as u32,
            });
        }
        result_text = email_regex
            .replace_all(&result_text, "[邮箱已隐藏]")
            .to_string();
    }

    // Bank card filter
    if bank_card {
        let bank_regex = Regex::new(r"\d{16,19}").unwrap();
        for mat in bank_regex.find_iter(&text) {
            // Only match if it looks like a bank card (starts with common prefixes)
            let matched = mat.as_str();
            if matched.starts_with("62") || matched.starts_with("4") || matched.starts_with("5") {
                filtered_items.push(FilteredItem {
                    item_type: "bank_card".to_string(),
                    position: mat.start() as u32,
                });
            }
        }
        result_text = bank_regex
            .replace_all(&result_text, |caps: &regex::Captures| {
                let matched = caps.get(0).unwrap().as_str();
                if matched.starts_with("62") || matched.starts_with("4") || matched.starts_with("5")
                {
                    "[银行卡已隐藏]".to_string()
                } else {
                    matched.to_string()
                }
            })
            .to_string();
    }

    FilterResult {
        text: result_text,
        filtered_count: filtered_items.len() as u32,
        filtered_items,
    }
}

/// Get the API version for compatibility checking
#[uniffi::export]
pub fn get_api_version() -> String {
    "1.0.0".to_string()
}

// UniFFI scaffolding
uniffi::setup_scaffolding!();

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_phone() {
        let result = filter_sensitive(
            "我的电话是13812345678".to_string(),
            true,
            false,
            false,
            false,
        );
        assert_eq!(result.text, "我的电话是[手机号已隐藏]");
        assert_eq!(result.filtered_count, 1);
    }

    #[test]
    fn test_filter_email() {
        let result = filter_sensitive(
            "邮箱: test@example.com".to_string(),
            false,
            false,
            true,
            false,
        );
        assert_eq!(result.text, "邮箱: [邮箱已隐藏]");
        assert_eq!(result.filtered_count, 1);
    }

    #[test]
    fn test_filter_disabled() {
        update_privacy_config(PrivacyConfig {
            enabled: false,
            ..Default::default()
        });

        let result = filter_with_config("电话13812345678".to_string());
        assert_eq!(result.text, "电话13812345678");
        assert_eq!(result.filtered_count, 0);

        // Reset
        update_privacy_config(PrivacyConfig::default());
    }
}
