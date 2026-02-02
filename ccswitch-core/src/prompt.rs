//! Prompt 数据结构模块
//!
//! 定义 Prompt 的数据结构，用于管理各应用的系统提示词。

use serde::{Deserialize, Serialize};

use crate::app_config::AppType;

/// Prompt 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    /// 唯一标识符
    pub id: String,
    /// 显示名称
    pub name: String,
    /// Prompt 内容
    pub content: String,
    /// 描述
    pub description: Option<String>,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间 (Unix 时间戳)
    pub created_at: Option<i64>,
    /// 更新时间 (Unix 时间戳)
    pub updated_at: Option<i64>,
}

impl Prompt {
    /// 创建新的 Prompt
    pub fn new(id: impl Into<String>, name: impl Into<String>, content: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: id.into(),
            name: name.into(),
            content: content.into(),
            description: None,
            enabled: false,
            created_at: Some(now),
            updated_at: Some(now),
        }
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 设置启用状态
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// 获取应用的 Prompt 文件路径
pub fn get_prompt_path(app: &AppType) -> std::path::PathBuf {
    use crate::config::{
        get_claude_config_dir, get_codex_config_dir, get_gemini_config_dir, get_opencode_config_dir,
    };

    match app {
        AppType::Claude => get_claude_config_dir().join("CLAUDE.md"),
        AppType::Codex => get_codex_config_dir().join("AGENTS.md"),
        AppType::Gemini => get_gemini_config_dir().join("GEMINI.md"),
        AppType::OpenCode => get_opencode_config_dir().join("AGENTS.md"),
    }
}

/// 获取应用的 Prompt 文件名
pub fn get_prompt_filename(app: &AppType) -> &'static str {
    match app {
        AppType::Claude => "CLAUDE.md",
        AppType::Codex => "AGENTS.md",
        AppType::Gemini => "GEMINI.md",
        AppType::OpenCode => "AGENTS.md",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_new() {
        let prompt = Prompt::new("test-prompt", "Test Prompt", "# Test Content");
        assert_eq!(prompt.id, "test-prompt");
        assert_eq!(prompt.name, "Test Prompt");
        assert_eq!(prompt.content, "# Test Content");
        assert!(!prompt.enabled);
    }

    #[test]
    fn test_prompt_with_description() {
        let prompt = Prompt::new("test", "Test", "content")
            .with_description("A test prompt")
            .with_enabled(true);

        assert_eq!(prompt.description, Some("A test prompt".to_string()));
        assert!(prompt.enabled);
    }
}