//! Skill 服务模块
//!
//! 提供 Skill 的业务逻辑，包括安装、同步到各应用等。

use indexmap::IndexMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::app_config::AppType;
use crate::config::{
    get_app_config_dir, get_claude_config_dir, get_codex_config_dir, get_gemini_config_dir,
    get_opencode_config_dir,
};
use crate::error::AppError;
use crate::skill::{Skill, SkillRepo};
use crate::store::AppState;

/// Skill 服务
pub struct SkillService;

impl SkillService {
    /// 获取 Skills 存储目录
    pub fn get_skills_dir() -> PathBuf {
        get_app_config_dir().join("skills")
    }

    /// 获取应用的 Skills 目录
    pub fn get_app_skills_dir(app: &AppType) -> PathBuf {
        match app {
            AppType::Claude => get_claude_config_dir().join("skills"),
            AppType::Codex => get_codex_config_dir().join("skills"),
            AppType::Gemini => get_gemini_config_dir().join("skills"),
            AppType::OpenCode => get_opencode_config_dir().join("skills"),
        }
    }

    /// 列出所有 Skills
    pub fn list(state: &AppState) -> Result<IndexMap<String, Skill>, AppError> {
        state.db.get_all_skills()
    }

    /// 获取单个 Skill
    pub fn get(state: &AppState, id: &str) -> Result<Option<Skill>, AppError> {
        state.db.get_skill(id)
    }

    /// 从 GitHub 仓库安装 Skill
    pub fn install(
        state: &AppState,
        repo: &str,
        branch: Option<String>,
    ) -> Result<Skill, AppError> {
        // 解析仓库格式 owner/name
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            return Err(AppError::InvalidInput(
                "仓库格式应为 owner/name".to_string(),
            ));
        }

        let owner = parts[0];
        let name = parts[1];
        let branch = branch.unwrap_or_else(|| "main".to_string());

        // 生成 Skill ID
        let skill_id = format!("{}-{}", owner, name);

        // 检查是否已安装
        if state.db.get_skill(&skill_id)?.is_some() {
            return Err(AppError::InvalidInput(format!(
                "Skill '{}' 已安装",
                skill_id
            )));
        }

        // 确保 Skills 目录存在
        let skills_dir = Self::get_skills_dir();
        fs::create_dir_all(&skills_dir).map_err(|e| AppError::io(&skills_dir, e))?;

        // 克隆仓库
        let skill_dir = skills_dir.join(&skill_id);
        let repo_url = format!("https://github.com/{}/{}.git", owner, name);

        let output = Command::new("git")
            .args(["clone", "--depth", "1", "--branch", &branch, &repo_url])
            .arg(&skill_dir)
            .output()
            .map_err(|e| AppError::Config(format!("执行 git clone 失败: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Config(format!("克隆仓库失败: {}", stderr)));
        }

        // 创建 Skill 记录
        let skill = Skill::new(&skill_id, name, skill_dir.to_string_lossy())
            .with_repo(owner, name, Some(branch));

        state.db.save_skill(&skill)?;

        Ok(skill)
    }

    /// 卸载 Skill
    pub fn uninstall(state: &AppState, id: &str) -> Result<(), AppError> {
        let skill = state
            .db
            .get_skill(id)?
            .ok_or_else(|| AppError::InvalidInput(format!("Skill '{}' 不存在", id)))?;

        // 删除本地目录
        let skill_dir = PathBuf::from(&skill.directory);
        if skill_dir.exists() {
            fs::remove_dir_all(&skill_dir).map_err(|e| AppError::io(&skill_dir, e))?;
        }

        // 删除所有应用中的 symlink
        for app in AppType::all() {
            Self::remove_app_symlink(app, id)?;
        }

        // 从数据库删除
        state.db.delete_skill(id)?;

        Ok(())
    }

    /// 切换 Skill 的应用启用状态
    pub fn toggle(state: &AppState, id: &str, app: AppType, enable: bool) -> Result<(), AppError> {
        let mut skill = state
            .db
            .get_skill(id)?
            .ok_or_else(|| AppError::InvalidInput(format!("Skill '{}' 不存在", id)))?;

        skill.apps.set_enabled_for(&app, enable);
        state.db.update_skill_apps(id, &skill.apps)?;

        // 同步到应用
        if enable {
            Self::create_app_symlink(&app, &skill)?;
        } else {
            Self::remove_app_symlink(&app, id)?;
        }

        Ok(())
    }

    /// 同步所有 Skills 到所有应用
    pub fn sync_all(state: &AppState) -> Result<(), AppError> {
        let skills = state.db.get_all_skills()?;

        for app in AppType::all() {
            // 确保应用 Skills 目录存在
            let app_skills_dir = Self::get_app_skills_dir(app);
            fs::create_dir_all(&app_skills_dir).map_err(|e| AppError::io(&app_skills_dir, e))?;

            for (_, skill) in &skills {
                if skill.apps.is_enabled_for(app) {
                    Self::create_app_symlink(app, skill)?;
                } else {
                    Self::remove_app_symlink(app, &skill.id)?;
                }
            }
        }

        Ok(())
    }

    /// 创建应用的 Skill symlink
    fn create_app_symlink(app: &AppType, skill: &Skill) -> Result<(), AppError> {
        let app_skills_dir = Self::get_app_skills_dir(app);
        fs::create_dir_all(&app_skills_dir).map_err(|e| AppError::io(&app_skills_dir, e))?;

        let link_path = app_skills_dir.join(&skill.id);
        let target_path = PathBuf::from(&skill.directory);

        // 如果已存在，先删除
        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path).or_else(|_| fs::remove_dir_all(&link_path)).ok();
        }

        // 创建 symlink
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target_path, &link_path)
                .map_err(|e| AppError::io(&link_path, e))?;
        }

        #[cfg(windows)]
        {
            // Windows 上使用目录 junction
            std::os::windows::fs::symlink_dir(&target_path, &link_path)
                .map_err(|e| AppError::io(&link_path, e))?;
        }

        Ok(())
    }

    /// 删除应用的 Skill symlink
    fn remove_app_symlink(app: &AppType, skill_id: &str) -> Result<(), AppError> {
        let app_skills_dir = Self::get_app_skills_dir(app);
        let link_path = app_skills_dir.join(skill_id);

        if link_path.exists() || link_path.is_symlink() {
            fs::remove_file(&link_path).or_else(|_| fs::remove_dir_all(&link_path)).ok();
        }

        Ok(())
    }

    /// 扫描本地 Skills 目录
    pub fn scan(state: &AppState) -> Result<Vec<String>, AppError> {
        let skills_dir = Self::get_skills_dir();

        if !skills_dir.exists() {
            return Ok(Vec::new());
        }

        let mut found = Vec::new();

        for entry in fs::read_dir(&skills_dir).map_err(|e| AppError::io(&skills_dir, e))? {
            let entry = entry.map_err(|e| AppError::Config(e.to_string()))?;
            let path = entry.path();

            if path.is_dir() {
                let skill_id = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                // 检查是否已在数据库中
                if state.db.get_skill(&skill_id)?.is_none() {
                    // 添加到数据库
                    let skill = Skill::new(&skill_id, &skill_id, path.to_string_lossy());
                    state.db.save_skill(&skill)?;
                    found.push(skill_id);
                }
            }
        }

        Ok(found)
    }

    // ===== Skill Repo 管理 =====

    /// 列出所有 Skill 仓库
    pub fn list_repos(state: &AppState) -> Result<Vec<SkillRepo>, AppError> {
        state.db.get_all_skill_repos()
    }

    /// 添加 Skill 仓库
    pub fn add_repo(state: &AppState, repo: SkillRepo) -> Result<(), AppError> {
        state.db.save_skill_repo(&repo)
    }

    /// 删除 Skill 仓库
    pub fn remove_repo(state: &AppState, id: &str) -> Result<(), AppError> {
        state.db.delete_skill_repo(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_service_list() {
        let state = AppState::memory().unwrap();
        let skills = SkillService::list(&state).unwrap();
        assert!(skills.is_empty());
    }
}
