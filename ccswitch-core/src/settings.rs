//! 本地设置管理模块
//!
//! 管理设备级别的本地设置，不随云同步。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::RwLock;

use crate::app_config::AppType;
use crate::config::{get_app_config_dir, read_json_file, write_json_file};
use crate::error::AppError;

/// 全局设置缓存
static SETTINGS_CACHE: OnceLock<RwLock<Option<AppSettings>>> = OnceLock::new();

fn settings_cache() -> &'static RwLock<Option<AppSettings>> {
    SETTINGS_CACHE.get_or_init(|| RwLock::new(None))
}

/// 主页面显示的应用配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VisibleApps {
    #[serde(default = "default_true")]
    pub claude: bool,
    #[serde(default = "default_true")]
    pub codex: bool,
    #[serde(default = "default_true")]
    pub gemini: bool,
    #[serde(default = "default_true")]
    pub opencode: bool,
}

fn default_true() -> bool {
    true
}

impl VisibleApps {
    pub fn is_visible(&self, app: &AppType) -> bool {
        match app {
            AppType::Claude => self.claude,
            AppType::Codex => self.codex,
            AppType::Gemini => self.gemini,
            AppType::OpenCode => self.opencode,
        }
    }
}

/// 应用设置结构
///
/// 存储设备级别设置，保存在本地 `~/.cc-switch/settings.json`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    // ===== 设备级目录覆盖 =====
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_config_dir: Option<String>,

    // ===== 设备级当前供应商 =====
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_claude: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_codex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider_gemini: Option<String>,

    // ===== 主页面显示的应用 =====
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_apps: Option<VisibleApps>,

    // ===== CLI/TUI 专用设置 =====
    /// 默认应用类型
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_app: Option<String>,

    /// 是否启用彩色输出
    #[serde(default = "default_true")]
    pub color_output: bool,

    /// 输出格式 (table, json, yaml)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

impl AppSettings {
    /// 获取设置文件路径
    pub fn path() -> PathBuf {
        get_app_config_dir().join("settings.json")
    }

    /// 加载设置
    pub fn load() -> Result<Self, AppError> {
        let path = Self::path();
        if path.exists() {
            read_json_file(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// 保存设置
    pub fn save(&self) -> Result<(), AppError> {
        write_json_file(&Self::path(), self)
    }

    /// 获取指定应用的当前供应商
    pub fn get_current_provider(&self, app_type: &AppType) -> Option<&str> {
        match app_type {
            AppType::Claude => self.current_provider_claude.as_deref(),
            AppType::Codex => self.current_provider_codex.as_deref(),
            AppType::Gemini => self.current_provider_gemini.as_deref(),
            AppType::OpenCode => None, // OpenCode 使用累加模式
        }
    }

    /// 设置指定应用的当前供应商
    pub fn set_current_provider(&mut self, app_type: &AppType, id: Option<&str>) {
        let value = id.map(|s| s.to_string());
        match app_type {
            AppType::Claude => self.current_provider_claude = value,
            AppType::Codex => self.current_provider_codex = value,
            AppType::Gemini => self.current_provider_gemini = value,
            AppType::OpenCode => {} // OpenCode 使用累加模式
        }
    }

    /// 获取指定应用的配置目录覆盖
    pub fn get_config_dir_override(&self, app_type: &AppType) -> Option<PathBuf> {
        let dir = match app_type {
            AppType::Claude => self.claude_config_dir.as_ref(),
            AppType::Codex => self.codex_config_dir.as_ref(),
            AppType::Gemini => self.gemini_config_dir.as_ref(),
            AppType::OpenCode => self.opencode_config_dir.as_ref(),
        };
        dir.map(PathBuf::from)
    }
}

/// 获取缓存的设置（带自动加载）
pub fn get_settings() -> Result<AppSettings, AppError> {
    let cache = settings_cache();
    
    // 尝试读取缓存
    {
        let read_guard = cache.read().map_err(|e| AppError::Lock(e.to_string()))?;
        if let Some(settings) = read_guard.as_ref() {
            return Ok(settings.clone());
        }
    }
    
    // 加载并缓存
    let settings = AppSettings::load()?;
    {
        let mut write_guard = cache.write().map_err(|e| AppError::Lock(e.to_string()))?;
        *write_guard = Some(settings.clone());
    }
    
    Ok(settings)
}

/// 更新设置（同时更新缓存和文件）
pub fn update_settings<F>(update_fn: F) -> Result<(), AppError>
where
    F: FnOnce(&mut AppSettings),
{
    let cache = settings_cache();
    let mut settings = get_settings()?;
    update_fn(&mut settings);
    settings.save()?;
    
    let mut write_guard = cache.write().map_err(|e| AppError::Lock(e.to_string()))?;
    *write_guard = Some(settings);
    
    Ok(())
}

/// 获取指定应用的当前供应商（便捷函数）
pub fn get_current_provider(app_type: &AppType) -> Option<String> {
    get_settings()
        .ok()
        .and_then(|s| s.get_current_provider(app_type).map(|s| s.to_string()))
}

/// 设置指定应用的当前供应商（便捷函数）
pub fn set_current_provider(app_type: &AppType, id: Option<&str>) -> Result<(), AppError> {
    update_settings(|settings| {
        settings.set_current_provider(app_type, id);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();
        assert!(settings.current_provider_claude.is_none());
        assert!(settings.color_output);
    }

    #[test]
    fn test_current_provider() {
        let mut settings = AppSettings::default();
        assert!(settings.get_current_provider(&AppType::Claude).is_none());

        settings.set_current_provider(&AppType::Claude, Some("provider-1"));
        assert_eq!(
            settings.get_current_provider(&AppType::Claude),
            Some("provider-1")
        );
    }
}
