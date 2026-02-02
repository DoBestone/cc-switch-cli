//! 环境变量冲突检测服务模块
//!
//! 检测可能与 AI CLI 工具冲突的环境变量。

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::app_config::AppType;
use crate::config::get_home_dir;
use crate::error::AppError;

/// 环境检测服务
pub struct EnvCheckerService;

/// 环境变量检测结果
#[derive(Debug, Clone)]
pub struct EnvCheckResult {
    pub app: AppType,
    pub conflicts: Vec<EnvConflict>,
}

/// 环境变量冲突
#[derive(Debug, Clone)]
pub struct EnvConflict {
    pub name: String,
    pub value: Option<String>,
    pub source: EnvSource,
    pub severity: ConflictSeverity,
    pub description: String,
}

/// 环境变量来源
#[derive(Debug, Clone, PartialEq)]
pub enum EnvSource {
    /// 进程环境变量
    Process,
    /// Shell 配置文件
    ShellConfig(String),
}

/// 冲突严重程度
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictSeverity {
    /// 警告
    Warning,
    /// 错误
    Error,
}

/// 各应用的关键环境变量
fn get_app_env_keywords(app: &AppType) -> Vec<(&'static str, &'static str)> {
    match app {
        AppType::Claude => vec![
            ("ANTHROPIC_API_KEY", "Anthropic API Key"),
            ("ANTHROPIC_AUTH_TOKEN", "Anthropic Auth Token"),
            ("ANTHROPIC_BASE_URL", "Anthropic Base URL"),
        ],
        AppType::Codex => vec![
            ("OPENAI_API_KEY", "OpenAI API Key"),
            ("OPENAI_BASE_URL", "OpenAI Base URL"),
            ("OPENAI_ORG_ID", "OpenAI Organization ID"),
        ],
        AppType::Gemini => vec![
            ("GEMINI_API_KEY", "Gemini API Key"),
            ("GOOGLE_GEMINI_API_KEY", "Google Gemini API Key"),
            ("GOOGLE_API_KEY", "Google API Key"),
        ],
        AppType::OpenCode => vec![
            ("OPENAI_API_KEY", "OpenAI API Key"),
            ("ANTHROPIC_API_KEY", "Anthropic API Key"),
        ],
    }
}

/// Shell 配置文件列表
fn get_shell_config_files() -> Vec<PathBuf> {
    let home = get_home_dir();
    vec![
        home.join(".bashrc"),
        home.join(".bash_profile"),
        home.join(".profile"),
        home.join(".zshrc"),
        home.join(".zprofile"),
        home.join(".config/fish/config.fish"),
    ]
}

impl EnvCheckerService {
    /// 检查指定应用的环境变量冲突
    pub fn check(app: AppType) -> Result<EnvCheckResult, AppError> {
        let keywords = get_app_env_keywords(&app);
        let mut conflicts = Vec::new();

        // 检查进程环境变量
        for (key, desc) in &keywords {
            if let Ok(value) = std::env::var(key) {
                conflicts.push(EnvConflict {
                    name: key.to_string(),
                    value: Some(Self::mask_value(&value)),
                    source: EnvSource::Process,
                    severity: ConflictSeverity::Warning,
                    description: format!("{} 已在环境变量中设置，可能覆盖配置文件设置", desc),
                });
            }
        }

        // 检查 Shell 配置文件
        for config_file in get_shell_config_files() {
            if !config_file.exists() {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&config_file) {
                for (key, desc) in &keywords {
                    if Self::check_env_in_content(&content, key) {
                        let file_name = config_file
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();

                        conflicts.push(EnvConflict {
                            name: key.to_string(),
                            value: None,
                            source: EnvSource::ShellConfig(file_name),
                            severity: ConflictSeverity::Warning,
                            description: format!("{} 在 Shell 配置文件中设置", desc),
                        });
                    }
                }
            }
        }

        Ok(EnvCheckResult { app, conflicts })
    }

    /// 检查所有应用的环境变量冲突
    pub fn check_all() -> Result<Vec<EnvCheckResult>, AppError> {
        let mut results = Vec::new();

        for app in AppType::all() {
            results.push(Self::check(*app)?);
        }

        Ok(results)
    }

    /// 列出指定应用的相关环境变量
    pub fn list_env_vars(app: AppType) -> Vec<(String, Option<String>)> {
        let keywords = get_app_env_keywords(&app);
        let mut vars = Vec::new();

        for (key, _) in keywords {
            let value = std::env::var(key).ok().map(|v| Self::mask_value(&v));
            vars.push((key.to_string(), value));
        }

        vars
    }

    /// 检查内容中是否包含环境变量设置
    fn check_env_in_content(content: &str, key: &str) -> bool {
        // 检查 export KEY= 或 KEY= 模式
        let patterns = [
            format!("export {}=", key),
            format!("export {} =", key),
            format!("{}=", key),
        ];

        for pattern in &patterns {
            if content.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// 掩码敏感值
    fn mask_value(value: &str) -> String {
        if value.len() <= 8 {
            "*".repeat(value.len())
        } else {
            format!("{}...{}", &value[..4], &value[value.len() - 4..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_value() {
        assert_eq!(EnvCheckerService::mask_value("short"), "*****");
        assert_eq!(
            EnvCheckerService::mask_value("sk-ant-api03-xxxxxxxxxxxxx"),
            "sk-a...xxxx"
        );
    }

    #[test]
    fn test_check_env_in_content() {
        let content = r#"
export ANTHROPIC_API_KEY="sk-xxx"
export PATH="/usr/bin:$PATH"
"#;
        assert!(EnvCheckerService::check_env_in_content(
            content,
            "ANTHROPIC_API_KEY"
        ));
        assert!(!EnvCheckerService::check_env_in_content(content, "OPENAI_API_KEY"));
    }
}
