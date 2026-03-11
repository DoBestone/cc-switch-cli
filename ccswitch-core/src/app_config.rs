//! 应用类型定义模块
//!
//! 定义支持的 AI CLI 应用类型。

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// 支持的应用类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppType {
    /// Claude Code CLI
    Claude,
    /// Codex CLI
    Codex,
    /// Gemini CLI
    Gemini,
    /// OpenCode CLI
    OpenCode,
    /// OpenClaw CLI
    OpenClaw,
}

impl AppType {
    /// 获取应用类型的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::OpenCode => "opencode",
            Self::OpenClaw => "openclaw",
        }
    }

    /// 获取应用的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex CLI",
            Self::Gemini => "Gemini CLI",
            Self::OpenCode => "OpenCode",
            Self::OpenClaw => "OpenClaw",
        }
    }

    /// 获取所有应用类型
    pub fn all() -> &'static [AppType] {
        &[Self::Claude, Self::Codex, Self::Gemini, Self::OpenCode, Self::OpenClaw]
    }

    /// 检查是否为累加模式（多供应商共存）
    ///
    /// OpenCode 和 OpenClaw 使用累加模式，其他应用使用独占模式
    pub fn is_additive_mode(&self) -> bool {
        matches!(self, Self::OpenCode | Self::OpenClaw)
    }

    /// 检查是否支持 MCP
    ///
    /// OpenClaw 不支持 MCP
    pub fn supports_mcp(&self) -> bool {
        !matches!(self, Self::OpenClaw)
    }

    /// 检查是否支持 Skills
    ///
    /// OpenClaw 不支持 Skills
    pub fn supports_skills(&self) -> bool {
        !matches!(self, Self::OpenClaw)
    }
}

impl fmt::Display for AppType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AppType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "claude-code" | "claude_code" => Ok(Self::Claude),
            "codex" | "codex-cli" | "codex_cli" => Ok(Self::Codex),
            "gemini" | "gemini-cli" | "gemini_cli" => Ok(Self::Gemini),
            "opencode" | "open-code" | "open_code" => Ok(Self::OpenCode),
            "openclaw" | "open-claw" | "open_claw" => Ok(Self::OpenClaw),
            _ => Err(format!("未知的应用类型: {}", s)),
        }
    }
}

/// MCP 服务器应用状态（标记应用到哪些客户端）
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpApps {
    #[serde(default)]
    pub claude: bool,
    #[serde(default)]
    pub codex: bool,
    #[serde(default)]
    pub gemini: bool,
    #[serde(default)]
    pub opencode: bool,
    // Note: OpenClaw does not support MCP
}

impl McpApps {
    /// 创建新实例
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查指定应用是否启用
    pub fn is_enabled_for(&self, app: &AppType) -> bool {
        match app {
            AppType::Claude => self.claude,
            AppType::Codex => self.codex,
            AppType::Gemini => self.gemini,
            AppType::OpenCode => self.opencode,
            AppType::OpenClaw => false, // OpenClaw doesn't support MCP
        }
    }

    /// 设置指定应用的启用状态
    pub fn set_enabled_for(&mut self, app: &AppType, enabled: bool) {
        match app {
            AppType::Claude => self.claude = enabled,
            AppType::Codex => self.codex = enabled,
            AppType::Gemini => self.gemini = enabled,
            AppType::OpenCode => self.opencode = enabled,
            AppType::OpenClaw => {} // OpenClaw doesn't support MCP, ignore
        }
    }

    /// 获取所有启用的应用列表
    pub fn enabled_apps(&self) -> Vec<AppType> {
        let mut apps = Vec::new();
        if self.claude {
            apps.push(AppType::Claude);
        }
        if self.codex {
            apps.push(AppType::Codex);
        }
        if self.gemini {
            apps.push(AppType::Gemini);
        }
        if self.opencode {
            apps.push(AppType::OpenCode);
        }
        apps
    }

    /// 检查是否所有应用都未启用
    pub fn is_empty(&self) -> bool {
        !self.claude && !self.codex && !self.gemini && !self.opencode
    }
}

/// Skill 应用状态（标记 Skill 应用到哪些客户端）
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillApps {
    #[serde(default)]
    pub claude: bool,
    #[serde(default)]
    pub codex: bool,
    #[serde(default)]
    pub gemini: bool,
    #[serde(default)]
    pub opencode: bool,
    // Note: OpenClaw does not support Skills
}

impl SkillApps {
    /// 创建新实例
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查指定应用是否启用
    pub fn is_enabled_for(&self, app: &AppType) -> bool {
        match app {
            AppType::Claude => self.claude,
            AppType::Codex => self.codex,
            AppType::Gemini => self.gemini,
            AppType::OpenCode => self.opencode,
            AppType::OpenClaw => false, // OpenClaw doesn't support Skills
        }
    }

    /// 设置指定应用的启用状态
    pub fn set_enabled_for(&mut self, app: &AppType, enabled: bool) {
        match app {
            AppType::Claude => self.claude = enabled,
            AppType::Codex => self.codex = enabled,
            AppType::Gemini => self.gemini = enabled,
            AppType::OpenCode => self.opencode = enabled,
            AppType::OpenClaw => {} // OpenClaw doesn't support Skills, ignore
        }
    }

    /// 获取所有启用的应用列表
    pub fn enabled_apps(&self) -> Vec<AppType> {
        let mut apps = Vec::new();
        if self.claude {
            apps.push(AppType::Claude);
        }
        if self.codex {
            apps.push(AppType::Codex);
        }
        if self.gemini {
            apps.push(AppType::Gemini);
        }
        if self.opencode {
            apps.push(AppType::OpenCode);
        }
        apps
    }

    /// 检查是否所有应用都未启用
    pub fn is_empty(&self) -> bool {
        !self.claude && !self.codex && !self.gemini && !self.opencode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_type_from_str() {
        assert_eq!(AppType::from_str("claude").unwrap(), AppType::Claude);
        assert_eq!(AppType::from_str("Claude").unwrap(), AppType::Claude);
        assert_eq!(AppType::from_str("codex").unwrap(), AppType::Codex);
        assert_eq!(AppType::from_str("gemini").unwrap(), AppType::Gemini);
        assert_eq!(AppType::from_str("opencode").unwrap(), AppType::OpenCode);
        assert_eq!(AppType::from_str("openclaw").unwrap(), AppType::OpenClaw);
        assert_eq!(AppType::from_str("OpenClaw").unwrap(), AppType::OpenClaw);
        assert!(AppType::from_str("unknown").is_err());
    }

    #[test]
    fn test_app_type_all() {
        let all = AppType::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&AppType::Claude));
        assert!(all.contains(&AppType::Codex));
        assert!(all.contains(&AppType::Gemini));
        assert!(all.contains(&AppType::OpenCode));
        assert!(all.contains(&AppType::OpenClaw));
    }

    #[test]
    fn test_app_type_additive_mode() {
        assert!(!AppType::Claude.is_additive_mode());
        assert!(!AppType::Codex.is_additive_mode());
        assert!(!AppType::Gemini.is_additive_mode());
        assert!(AppType::OpenCode.is_additive_mode());
        assert!(AppType::OpenClaw.is_additive_mode());
    }

    #[test]
    fn test_app_type_supports_mcp() {
        assert!(AppType::Claude.supports_mcp());
        assert!(AppType::Codex.supports_mcp());
        assert!(AppType::Gemini.supports_mcp());
        assert!(AppType::OpenCode.supports_mcp());
        assert!(!AppType::OpenClaw.supports_mcp());
    }

    #[test]
    fn test_app_type_supports_skills() {
        assert!(AppType::Claude.supports_skills());
        assert!(AppType::Codex.supports_skills());
        assert!(AppType::Gemini.supports_skills());
        assert!(AppType::OpenCode.supports_skills());
        assert!(!AppType::OpenClaw.supports_skills());
    }

    #[test]
    fn test_mcp_apps() {
        let mut apps = McpApps::new();
        assert!(apps.is_empty());

        apps.set_enabled_for(&AppType::Claude, true);
        assert!(apps.is_enabled_for(&AppType::Claude));
        assert!(!apps.is_empty());

        // OpenClaw should not be enabled for MCP
        apps.set_enabled_for(&AppType::OpenClaw, true);
        assert!(!apps.is_enabled_for(&AppType::OpenClaw));

        let enabled = apps.enabled_apps();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0], AppType::Claude);
    }

    #[test]
    fn test_skill_apps() {
        let mut apps = SkillApps::new();
        assert!(apps.is_empty());

        apps.set_enabled_for(&AppType::Claude, true);
        assert!(apps.is_enabled_for(&AppType::Claude));
        assert!(!apps.is_empty());

        // OpenClaw should not be enabled for Skills
        apps.set_enabled_for(&AppType::OpenClaw, true);
        assert!(!apps.is_enabled_for(&AppType::OpenClaw));

        let enabled = apps.enabled_apps();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0], AppType::Claude);
    }
}
