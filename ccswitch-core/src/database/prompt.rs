//! Prompt 数据库操作模块

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::prompt::Prompt;
use indexmap::IndexMap;

impl Database {
    // ===== Prompt DAO =====

    /// 获取指定应用的所有 Prompts
    pub fn get_all_prompts(&self, app_type: &str) -> Result<IndexMap<String, Prompt>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, name, content, description, enabled, created_at, updated_at
                FROM prompts
                WHERE app_type = ?
                ORDER BY created_at ASC
                "#,
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let prompts = stmt
            .query_map([app_type], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let content: String = row.get(2)?;
                let description: Option<String> = row.get(3)?;
                let enabled: bool = row.get::<_, i64>(4)? != 0;
                let created_at: Option<i64> = row.get(5)?;
                let updated_at: Option<i64> = row.get(6)?;

                Ok((
                    id.clone(),
                    Prompt {
                        id,
                        name,
                        content,
                        description,
                        enabled,
                        created_at,
                        updated_at,
                    },
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut result = IndexMap::new();
        for prompt_result in prompts {
            let (id, prompt) = prompt_result.map_err(|e| AppError::Database(e.to_string()))?;
            result.insert(id, prompt);
        }

        Ok(result)
    }

    /// 获取单个 Prompt
    pub fn get_prompt(&self, app_type: &str, id: &str) -> Result<Option<Prompt>, AppError> {
        let prompts = self.get_all_prompts(app_type)?;
        Ok(prompts.get(id).cloned())
    }

    /// 保存 Prompt
    pub fn save_prompt(&self, app_type: &str, prompt: &Prompt) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            r#"
            INSERT OR REPLACE INTO prompts
            (id, app_type, name, content, description, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                prompt.id,
                app_type,
                prompt.name,
                prompt.content,
                prompt.description,
                prompt.enabled as i64,
                prompt.created_at,
                prompt.updated_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 删除 Prompt
    pub fn delete_prompt(&self, app_type: &str, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "DELETE FROM prompts WHERE id = ? AND app_type = ?",
            rusqlite::params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 更新 Prompt 启用状态
    pub fn update_prompt_enabled(
        &self,
        app_type: &str,
        id: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE prompts SET enabled = ?, updated_at = ? WHERE id = ? AND app_type = ?",
            rusqlite::params![enabled as i64, now, id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 获取启用的 Prompt
    pub fn get_enabled_prompt(&self, app_type: &str) -> Result<Option<Prompt>, AppError> {
        let prompts = self.get_all_prompts(app_type)?;
        Ok(prompts.into_values().find(|p| p.enabled))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_crud() {
        let db = Database::memory().unwrap();

        let prompt = Prompt::new("test-prompt", "Test Prompt", "# Test Content");
        db.save_prompt("claude", &prompt).unwrap();

        let prompts = db.get_all_prompts("claude").unwrap();
        assert_eq!(prompts.len(), 1);
        assert!(prompts.contains_key("test-prompt"));

        db.delete_prompt("claude", "test-prompt").unwrap();
        let prompts = db.get_all_prompts("claude").unwrap();
        assert!(prompts.is_empty());
    }

    #[test]
    fn test_prompt_enable() {
        let db = Database::memory().unwrap();

        let prompt = Prompt::new("test-prompt", "Test Prompt", "# Test Content");
        db.save_prompt("claude", &prompt).unwrap();

        db.update_prompt_enabled("claude", "test-prompt", true)
            .unwrap();

        let prompt = db.get_prompt("claude", "test-prompt").unwrap().unwrap();
        assert!(prompt.enabled);

        let enabled = db.get_enabled_prompt("claude").unwrap();
        assert!(enabled.is_some());
        assert_eq!(enabled.unwrap().id, "test-prompt");
    }
}
