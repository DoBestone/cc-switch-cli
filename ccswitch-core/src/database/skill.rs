//! Skill 数据库操作模块

use crate::app_config::McpApps;
use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::skill::{Skill, SkillRepo};
use indexmap::IndexMap;

impl Database {
    // ===== Skill DAO =====

    /// 获取所有 Skills
    pub fn get_all_skills(&self) -> Result<IndexMap<String, Skill>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, name, description, directory, repo_owner, repo_name, repo_branch,
                       readme_url, enabled_claude, enabled_codex, enabled_gemini, enabled_opencode,
                       installed_at
                FROM skills
                ORDER BY installed_at ASC
                "#,
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let skills = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let description: Option<String> = row.get(2)?;
                let directory: String = row.get(3)?;
                let repo_owner: Option<String> = row.get(4)?;
                let repo_name: Option<String> = row.get(5)?;
                let repo_branch: Option<String> = row.get(6)?;
                let readme_url: Option<String> = row.get(7)?;
                let enabled_claude: bool = row.get::<_, i64>(8)? != 0;
                let enabled_codex: bool = row.get::<_, i64>(9)? != 0;
                let enabled_gemini: bool = row.get::<_, i64>(10)? != 0;
                let enabled_opencode: bool = row.get::<_, i64>(11)? != 0;
                let installed_at: Option<i64> = row.get(12)?;

                Ok((
                    id.clone(),
                    Skill {
                        id,
                        name,
                        description,
                        directory,
                        repo_owner,
                        repo_name,
                        repo_branch,
                        readme_url,
                        apps: McpApps {
                            claude: enabled_claude,
                            codex: enabled_codex,
                            gemini: enabled_gemini,
                            opencode: enabled_opencode,
                        },
                        installed_at,
                    },
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut result = IndexMap::new();
        for skill_result in skills {
            let (id, skill) = skill_result.map_err(|e| AppError::Database(e.to_string()))?;
            result.insert(id, skill);
        }

        Ok(result)
    }

    /// 获取单个 Skill
    pub fn get_skill(&self, id: &str) -> Result<Option<Skill>, AppError> {
        let skills = self.get_all_skills()?;
        Ok(skills.get(id).cloned())
    }

    /// 保存 Skill
    pub fn save_skill(&self, skill: &Skill) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            r#"
            INSERT OR REPLACE INTO skills
            (id, name, description, directory, repo_owner, repo_name, repo_branch,
             readme_url, enabled_claude, enabled_codex, enabled_gemini, enabled_opencode,
             installed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                skill.id,
                skill.name,
                skill.description,
                skill.directory,
                skill.repo_owner,
                skill.repo_name,
                skill.repo_branch,
                skill.readme_url,
                skill.apps.claude as i64,
                skill.apps.codex as i64,
                skill.apps.gemini as i64,
                skill.apps.opencode as i64,
                skill.installed_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 删除 Skill
    pub fn delete_skill(&self, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute("DELETE FROM skills WHERE id = ?", rusqlite::params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 更新 Skill 的应用启用状态
    pub fn update_skill_apps(&self, id: &str, apps: &McpApps) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            r#"
            UPDATE skills
            SET enabled_claude = ?, enabled_codex = ?, enabled_gemini = ?, enabled_opencode = ?
            WHERE id = ?
            "#,
            rusqlite::params![
                apps.claude as i64,
                apps.codex as i64,
                apps.gemini as i64,
                apps.opencode as i64,
                id,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ===== Skill Repo DAO =====

    /// 获取所有 Skill 仓库
    pub fn get_all_skill_repos(&self) -> Result<Vec<SkillRepo>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare("SELECT id, owner, name, branch, enabled FROM skill_repos")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let repos = stmt
            .query_map([], |row| {
                Ok(SkillRepo {
                    id: row.get(0)?,
                    owner: row.get(1)?,
                    name: row.get(2)?,
                    branch: row.get(3)?,
                    enabled: row.get::<_, i64>(4)? != 0,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut result = Vec::new();
        for repo_result in repos {
            result.push(repo_result.map_err(|e| AppError::Database(e.to_string()))?);
        }

        Ok(result)
    }

    /// 保存 Skill 仓库
    pub fn save_skill_repo(&self, repo: &SkillRepo) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "INSERT OR REPLACE INTO skill_repos (id, owner, name, branch, enabled) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![repo.id, repo.owner, repo.name, repo.branch, repo.enabled as i64],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 删除 Skill 仓库
    pub fn delete_skill_repo(&self, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute("DELETE FROM skill_repos WHERE id = ?", rusqlite::params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_crud() {
        let db = Database::memory().unwrap();

        let skill = Skill::new("test-skill", "Test Skill", "/path/to/skill");
        db.save_skill(&skill).unwrap();

        let skills = db.get_all_skills().unwrap();
        assert_eq!(skills.len(), 1);
        assert!(skills.contains_key("test-skill"));

        db.delete_skill("test-skill").unwrap();
        let skills = db.get_all_skills().unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_skill_repo_crud() {
        let db = Database::memory().unwrap();

        let repo = SkillRepo::new("owner", "repo");
        db.save_skill_repo(&repo).unwrap();

        let repos = db.get_all_skill_repos().unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "owner/repo");

        db.delete_skill_repo("owner/repo").unwrap();
        let repos = db.get_all_skill_repos().unwrap();
        assert!(repos.is_empty());
    }
}
