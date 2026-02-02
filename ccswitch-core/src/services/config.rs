//! 配置服务模块
//!
//! 处理应用配置的导入导出和路径管理。

use std::path::PathBuf;

use crate::app_config::AppType;
use crate::config::{
    get_app_config_dir, get_claude_config_dir, get_claude_mcp_path, get_claude_settings_path,
    get_codex_auth_path, get_codex_config_dir, get_codex_config_path, get_gemini_config_dir,
    get_gemini_settings_path, get_opencode_config_dir,
};
use crate::error::AppError;

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Yaml,
    Toml,
}

impl std::str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "yaml" | "yml" => Ok(Self::Yaml),
            "toml" => Ok(Self::Toml),
            _ => Err(format!("未知的导出格式: {}", s)),
        }
    }
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Yaml => write!(f, "yaml"),
            Self::Toml => write!(f, "toml"),
        }
    }
}

/// 配置路径信息
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    /// 应用配置目录
    pub app_config_dir: PathBuf,
    /// 数据库文件路径
    pub database_path: PathBuf,
    /// 设置文件路径
    pub settings_path: PathBuf,
}

/// 应用特定的配置路径
#[derive(Debug, Clone)]
pub struct AppConfigPaths {
    /// 配置目录
    pub config_dir: PathBuf,
    /// 主配置文件
    pub settings_path: PathBuf,
    /// MCP 配置文件（如有）
    pub mcp_path: Option<PathBuf>,
    /// 认证文件（如有）
    pub auth_path: Option<PathBuf>,
}

/// 配置服务
pub struct ConfigService;

impl ConfigService {
    /// 获取 cc-switch 配置路径信息
    pub fn get_paths() -> ConfigPaths {
        let app_config_dir = get_app_config_dir();
        ConfigPaths {
            database_path: app_config_dir.join("cc-switch.db"),
            settings_path: app_config_dir.join("settings.json"),
            app_config_dir,
        }
    }

    /// 获取指定应用的配置路径
    pub fn get_app_paths(app_type: AppType) -> AppConfigPaths {
        match app_type {
            AppType::Claude => AppConfigPaths {
                config_dir: get_claude_config_dir(),
                settings_path: get_claude_settings_path(),
                mcp_path: Some(get_claude_mcp_path()),
                auth_path: None,
            },
            AppType::Codex => AppConfigPaths {
                config_dir: get_codex_config_dir(),
                settings_path: get_codex_config_path(),
                mcp_path: None,
                auth_path: Some(get_codex_auth_path()),
            },
            AppType::Gemini => AppConfigPaths {
                config_dir: get_gemini_config_dir(),
                settings_path: get_gemini_settings_path(),
                mcp_path: None,
                auth_path: None,
            },
            AppType::OpenCode => AppConfigPaths {
                config_dir: get_opencode_config_dir(),
                settings_path: get_opencode_config_dir().join("config.toml"),
                mcp_path: None,
                auth_path: None,
            },
        }
    }

    /// 检查应用是否已安装/配置
    pub fn is_app_configured(app_type: AppType) -> bool {
        let paths = Self::get_app_paths(app_type);
        paths.config_dir.exists() || paths.settings_path.exists()
    }

    /// 获取所有已配置的应用
    pub fn get_configured_apps() -> Vec<AppType> {
        AppType::all()
            .iter()
            .copied()
            .filter(|app| Self::is_app_configured(*app))
            .collect()
    }

    /// 导出配置
    pub fn export_config(
        _app_type: Option<AppType>,
        _format: ExportFormat,
        _output_path: &PathBuf,
    ) -> Result<(), AppError> {
        // TODO: 实现配置导出
        Err(AppError::Message("导出功能尚未实现".to_string()))
    }

    /// 导入配置
    pub fn import_config(_input_path: &PathBuf) -> Result<(), AppError> {
        // TODO: 实现配置导入
        Err(AppError::Message("导入功能尚未实现".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_parse() {
        assert_eq!("json".parse::<ExportFormat>().unwrap(), ExportFormat::Json);
        assert_eq!("yaml".parse::<ExportFormat>().unwrap(), ExportFormat::Yaml);
        assert_eq!("yml".parse::<ExportFormat>().unwrap(), ExportFormat::Yaml);
        assert_eq!("toml".parse::<ExportFormat>().unwrap(), ExportFormat::Toml);
        assert!("unknown".parse::<ExportFormat>().is_err());
    }

    #[test]
    fn test_get_paths() {
        let paths = ConfigService::get_paths();
        assert!(paths.app_config_dir.ends_with(".cc-switch") || paths.app_config_dir.ends_with("cc-switch"));
    }
}
