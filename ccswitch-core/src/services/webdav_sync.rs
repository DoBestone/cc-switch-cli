//! WebDAV 同步服务
//!
//! 支持将配置同步到 WebDAV 服务器，实现跨设备同步。

use crate::error::AppError;
use crate::store::AppState;
use serde::{Deserialize, Serialize};

/// WebDAV 同步设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavSyncSettings {
    pub enabled: bool,
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub remote_root: String,
    pub profile: String,
    #[serde(default)]
    pub auto_sync_enabled: bool,
    #[serde(default)]
    pub sync_interval_minutes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl Default for WebDavSyncSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            username: String::new(),
            password: String::new(),
            remote_root: "cc-switch-sync".to_string(),
            profile: "default".to_string(),
            auto_sync_enabled: false,
            sync_interval_minutes: 30,
            last_sync_at: None,
            last_error: None,
        }
    }
}

impl WebDavSyncSettings {
    /// 验证配置
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled {
            if self.base_url.is_empty() {
                return Err("WebDAV URL 不能为空".to_string());
            }
            if self.username.is_empty() {
                return Err("用户名不能为空".to_string());
            }
        }
        Ok(())
    }

    /// 规范化配置
    pub fn normalize(&mut self) {
        // 移除末尾斜杠
        while self.base_url.ends_with('/') {
            self.base_url.pop();
        }
    }
}

/// 同步状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub last_sync_at: Option<i64>,
    pub last_error: Option<String>,
    pub sync_count: u64,
}

/// WebDAV 同步服务
pub struct WebDavSyncService;

impl WebDavSyncService {
    /// 获取设置
    pub fn get_settings(state: &AppState) -> Result<Option<WebDavSyncSettings>, AppError> {
        let settings_str = state.db.get_setting("webdav_sync_settings")?;

        if let Some(s) = settings_str {
            let settings: WebDavSyncSettings = serde_json::from_str(&s)
                .map_err(|e| AppError::Config(format!("解析 WebDAV 配置失败: {}", e)))?;
            Ok(Some(settings))
        } else {
            Ok(None)
        }
    }

    /// 保存设置
    pub fn save_settings(state: &AppState, settings: &WebDavSyncSettings) -> Result<(), AppError> {
        settings.validate()
            .map_err(|e| AppError::InvalidInput(e))?;

        let settings_str = serde_json::to_string(settings)
            .map_err(|e| AppError::Config(format!("序列化配置失败: {}", e)))?;

        state.db.set_setting("webdav_sync_settings", &settings_str)
    }

    /// 测试连接
    pub async fn test_connection(settings: &WebDavSyncSettings) -> Result<bool, AppError> {
        if settings.base_url.is_empty() {
            return Err(AppError::Message("WebDAV URL 未配置".to_string()));
        }

        let client = reqwest::Client::new();

        let response = client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &settings.base_url)
            .basic_auth(&settings.username, Some(&settings.password))
            .header("Depth", "0")
            .send()
            .await?;

        if response.status().is_success() || response.status().as_u16() == 207 {
            Ok(true)
        } else {
            Err(AppError::Http(format!("连接失败: HTTP {}", response.status())))
        }
    }

    /// 上传配置
    pub async fn upload(state: &AppState) -> Result<(), AppError> {
        let settings = Self::get_settings(state)?
            .ok_or_else(|| AppError::Message("WebDAV 同步未配置".to_string()))?;

        if !settings.enabled {
            return Err(AppError::Message("WebDAV 同步未启用".to_string()));
        }

        // 导出所有配置
        let config = crate::services::ConfigService::export_all(state)?;

        let content = serde_json::to_string_pretty(&config)
            .map_err(|e| AppError::Config(format!("序列化配置失败: {}", e)))?;

        let url = format!(
            "{}/{}/config.json",
            settings.base_url.trim_end_matches('/'),
            settings.remote_root
        );

        let client = reqwest::Client::new();

        let response = client
            .put(&url)
            .basic_auth(&settings.username, Some(&settings.password))
            .header("Content-Type", "application/json")
            .body(content)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::Http(format!("上传失败: HTTP {}", response.status())));
        }

        Ok(())
    }

    /// 下载配置
    pub async fn download(state: &AppState) -> Result<serde_json::Value, AppError> {
        let settings = Self::get_settings(state)?
            .ok_or_else(|| AppError::Message("WebDAV 同步未配置".to_string()))?;

        if !settings.enabled {
            return Err(AppError::Message("WebDAV 同步未启用".to_string()));
        }

        let url = format!(
            "{}/{}/config.json",
            settings.base_url.trim_end_matches('/'),
            settings.remote_root
        );

        let client = reqwest::Client::new();

        let response = client
            .get(&url)
            .basic_auth(&settings.username, Some(&settings.password))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::Http(format!("下载失败: HTTP {}", response.status())));
        }

        let content = response.text().await?;

        let config: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| AppError::Config(format!("解析配置失败: {}", e)))?;

        Ok(config)
    }

    /// 获取远程信息
    pub async fn fetch_remote_info(settings: &WebDavSyncSettings) -> Result<Option<serde_json::Value>, AppError> {
        let url = format!(
            "{}/{}/config.json",
            settings.base_url.trim_end_matches('/'),
            settings.remote_root
        );

        let client = reqwest::Client::new();

        let response = client
            .request(reqwest::Method::HEAD, &url)
            .basic_auth(&settings.username, Some(&settings.password))
            .send()
            .await?;

        if response.status().is_success() {
            let last_modified = response.headers()
                .get("last-modified")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            Ok(Some(serde_json::json!({
                "exists": true,
                "lastModified": last_modified
            })))
        } else {
            Ok(Some(serde_json::json!({
                "exists": false
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = WebDavSyncSettings::default();
        assert!(!settings.enabled);
        assert!(settings.base_url.is_empty());
    }

    #[test]
    fn test_validate() {
        let mut settings = WebDavSyncSettings::default();
        settings.enabled = true;
        assert!(settings.validate().is_err());

        settings.base_url = "https://dav.example.com".to_string();
        settings.username = "user".to_string();
        assert!(settings.validate().is_ok());
    }
}