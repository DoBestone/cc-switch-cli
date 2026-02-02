//! Skill 数据结构模块
//!
//! 定义 Skill 的数据结构，用于管理各应用的技能扩展。

use serde::{Deserialize, Serialize};

use crate::app_config::McpApps;

/// Skill 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// 唯一标识符
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 本地目录路径
    pub directory: String,
    /// GitHub 仓库所有者
    pub repo_owner: Option<String>,
    /// GitHub 仓库名称
    pub repo_name: Option<String>,
    /// GitHub 仓库分支
    pub repo_branch: Option<String>,
    /// README URL
    pub readme_url: Option<String>,
    /// 应用启用状态
    #[serde(default)]
    pub apps: McpApps,
    /// 安装时间 (Unix 时间戳)
    pub installed_at: Option<i64>,
}

impl Skill {
    /// 创建新的 Skill
    pub fn new(id: impl Into<String>, name: impl Into<String>, directory: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            directory: directory.into(),
            repo_owner: None,
            repo_name: None,
            repo_branch: None,
            readme_url: None,
            apps: McpApps::default(),
            installed_at: Some(chrono::Utc::now().timestamp()),
        }
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 设置 GitHub 仓库信息
    pub fn with_repo(
        mut self,
        owner: impl Into<String>,
        name: impl Into<String>,
        branch: Option<String>,
    ) -> Self {
        self.repo_owner = Some(owner.into());
        self.repo_name = Some(name.into());
        self.repo_branch = branch;
        self
    }

    /// 获取 GitHub 仓库 URL
    pub fn repo_url(&self) -> Option<String> {
        match (&self.repo_owner, &self.repo_name) {
            (Some(owner), Some(name)) => Some(format!("https://github.com/{}/{}", owner, name)),
            _ => None,
        }
    }

    /// 获取启用的应用列表字符串
    pub fn enabled_apps_str(&self) -> String {
        let apps = self.apps.enabled_apps();
        if apps.is_empty() {
            "无".to_string()
        } else {
            apps.iter()
                .map(|a| a.display_name())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

/// Skill 仓库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRepo {
    /// 唯一标识符
    pub id: String,
    /// GitHub 仓库所有者
    pub owner: String,
    /// GitHub 仓库名称
    pub name: String,
    /// 分支
    pub branch: String,
    /// 是否启用
    pub enabled: bool,
}

impl SkillRepo {
    /// 创建新的 Skill 仓库
    pub fn new(owner: impl Into<String>, name: impl Into<String>) -> Self {
        let owner = owner.into();
        let name = name.into();
        let id = format!("{}/{}", owner, name);

        Self {
            id,
            owner,
            name,
            branch: "main".to_string(),
            enabled: true,
        }
    }

    /// 设置分支
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = branch.into();
        self
    }

    /// 获取仓库 URL
    pub fn url(&self) -> String {
        format!("https://github.com/{}/{}", self.owner, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_new() {
        let skill = Skill::new("test-skill", "Test Skill", "/path/to/skill");
        assert_eq!(skill.id, "test-skill");
        assert_eq!(skill.name, "Test Skill");
        assert_eq!(skill.directory, "/path/to/skill");
    }

    #[test]
    fn test_skill_repo_url() {
        let skill = Skill::new("test", "Test", "/path")
            .with_repo("owner", "repo", Some("main".to_string()));

        assert_eq!(skill.repo_url(), Some("https://github.com/owner/repo".to_string()));
    }

    #[test]
    fn test_skill_repo_new() {
        let repo = SkillRepo::new("anthropics", "claude-code-skills");
        assert_eq!(repo.id, "anthropics/claude-code-skills");
        assert_eq!(repo.url(), "https://github.com/anthropics/claude-code-skills");
    }
}