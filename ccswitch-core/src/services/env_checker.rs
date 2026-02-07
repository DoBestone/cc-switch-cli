//! 环境变量冲突检测服务模块
//!
//! 检测可能与 AI CLI 工具冲突的环境变量。

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

    /// 备份 Shell 配置文件
    pub fn backup_shell_configs() -> Result<PathBuf, AppError> {
        let home = get_home_dir();
        let backup_dir = home.join(".cc-switch-backups");

        // 创建备份目录
        fs::create_dir_all(&backup_dir).map_err(|e| {
            AppError::Message(format!("无法创建备份目录: {}", e))
        })?;

        // 生成时间戳
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_subdir = backup_dir.join(format!("env_backup_{}", timestamp));
        fs::create_dir_all(&backup_subdir).map_err(|e| {
            AppError::Message(format!("无法创建备份子目录: {}", e))
        })?;

        // 备份所有 Shell 配置文件
        for config_file in get_shell_config_files() {
            if config_file.exists() {
                let file_name = config_file.file_name()
                    .ok_or_else(|| AppError::Config("无效的文件名".to_string()))?;
                let backup_file = backup_subdir.join(file_name);

                fs::copy(&config_file, &backup_file).map_err(|e| {
                    AppError::Message(format!("无法备份文件 {:?}: {}", config_file, e))
                })?;
            }
        }

        Ok(backup_subdir)
    }

    /// 从 Shell 配置文件中移除指定的环境变量
    pub fn remove_env_from_shell_configs(app: AppType) -> Result<Vec<String>, AppError> {
        let keywords = get_app_env_keywords(&app);
        let mut modified_files = Vec::new();

        for config_file in get_shell_config_files() {
            if !config_file.exists() {
                continue;
            }

            let content = fs::read_to_string(&config_file).map_err(|e| {
                AppError::Message(format!("无法读取文件 {:?}: {}", config_file, e))
            })?;

            let mut modified = false;
            let mut new_lines = Vec::new();
            let mut skip_next = false;

            for line in content.lines() {
                if skip_next {
                    skip_next = false;
                    continue;
                }

                let mut should_skip = false;
                let trimmed: &str = line.trim();

                // 检查是否是要移除的环境变量
                for (key, _) in &keywords {
                    if trimmed.starts_with(&format!("export {}=", key))
                        || trimmed.starts_with(&format!("export {} =", key))
                        || (trimmed.starts_with(&format!("{}=", key)) && !trimmed.contains("PATH"))
                    {
                        should_skip = true;
                        modified = true;
                        break;
                    }
                }

                if !should_skip {
                    new_lines.push(line);
                }
            }

            if modified {
                let new_content = new_lines.join("\n") + "\n";
                fs::write(&config_file, new_content).map_err(|e| {
                    AppError::Message(format!("无法写入文件 {:?}: {}", config_file, e))
                })?;

                modified_files.push(
                    config_file.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                );
            }
        }

        Ok(modified_files)
    }

    /// 列出所有备份
    pub fn list_backups() -> Result<Vec<PathBuf>, AppError> {
        let home = get_home_dir();
        let backup_dir = home.join(".cc-switch-backups");

        if !backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut backups: Vec<PathBuf> = Vec::new();
        for entry in fs::read_dir(&backup_dir).map_err(|e| {
            AppError::Message(format!("无法读取备份目录: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                AppError::Message(format!("无法读取目录项: {}", e))
            })?;
            let path: PathBuf = entry.path();
            if path.is_dir() {
                backups.push(path);
            }
        }

        // 按时间倒序排序
        backups.sort_by(|a: &PathBuf, b: &PathBuf| b.cmp(a));
        Ok(backups)
    }

    /// 恢复备份
    pub fn restore_backup(backup_path: &PathBuf) -> Result<Vec<String>, AppError> {
        if !backup_path.exists() {
            return Err(AppError::Config("备份目录不存在".to_string()));
        }

        let mut restored_files = Vec::new();
        let home = get_home_dir();

        for entry in fs::read_dir(backup_path).map_err(|e| {
            AppError::Message(format!("无法读取备份目录: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                AppError::Message(format!("无法读取目录项: {}", e))
            })?;

            let backup_file: PathBuf = entry.path();
            if backup_file.is_file() {
                let file_name = backup_file.file_name()
                    .ok_or_else(|| AppError::Config("无效的文件名".to_string()))?;

                // 确定恢复目标路径
                let target_path = if file_name.to_str().unwrap_or("").contains("fish") {
                    home.join(".config/fish").join(file_name)
                } else {
                    home.join(file_name)
                };

                // 确保目标目录存在
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        AppError::Message(format!("无法创建目录: {}", e))
                    })?;
                }

                fs::copy(&backup_file, &target_path).map_err(|e| {
                    AppError::Message(format!("无法恢复文件 {:?}: {}", backup_file, e))
                })?;

                restored_files.push(file_name.to_string_lossy().to_string());
            }
        }

        Ok(restored_files)
    }

    /// 清除当前 Shell 会话中的环境变量（生成脚本）
    pub fn generate_unset_script(app: AppType) -> String {
        let keywords = get_app_env_keywords(&app);
        let mut script = String::new();

        script.push_str("#!/bin/bash\n");
        script.push_str("# CC-Switch 环境变量清除脚本\n");
        script.push_str(&format!("# 应用: {}\n\n", app.display_name()));

        for (key, desc) in keywords {
            script.push_str(&format!("# {}\n", desc));
            script.push_str(&format!("unset {}\n", key));
        }

        script.push_str("\necho \"环境变量已清除，请重启终端以确保生效\"\n");
        script
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
