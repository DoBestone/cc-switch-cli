//! 数据库模块 - SQLite 数据持久化
//!
//! 提供应用的核心数据存储功能，包括：
//! - 供应商配置管理
//! - MCP 服务器配置
//! - 通用设置存储

mod schema;

use crate::config::get_app_config_dir;
use crate::error::AppError;
use crate::provider::Provider;
use indexmap::IndexMap;
use rusqlite::Connection;
use serde::Serialize;
use std::sync::Mutex;

/// 当前 Schema 版本号
pub(crate) const SCHEMA_VERSION: i32 = 5;

/// 安全地序列化 JSON
pub(crate) fn to_json_string<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value)
        .map_err(|e| AppError::Config(format!("JSON serialization failed: {e}")))
}

/// 安全地获取 Mutex 锁
macro_rules! lock_conn {
    ($mutex:expr) => {
        $mutex
            .lock()
            .map_err(|e| AppError::Database(format!("Mutex lock failed: {}", e)))?
    };
}

pub(crate) use lock_conn;

/// 数据库连接封装
pub struct Database {
    pub(crate) conn: Mutex<Connection>,
}

impl Database {
    /// 初始化数据库连接
    ///
    /// 数据库文件位于 `~/.cc-switch/cc-switch.db`
    pub fn init() -> Result<Self, AppError> {
        let db_path = get_app_config_dir().join("cc-switch.db");

        // 确保父目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
        }

        let conn = Connection::open(&db_path).map_err(|e| AppError::Database(e.to_string()))?;

        // 启用外键约束
        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;

        Ok(db)
    }

    /// 创建内存数据库（用于测试）
    pub fn memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory().map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(e.to_string()))?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;

        Ok(db)
    }

    /// 创建数据表
    fn create_tables(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute_batch(
            r#"
            -- 供应商表
            CREATE TABLE IF NOT EXISTS providers (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                settings_config TEXT NOT NULL,
                website_url TEXT,
                category TEXT,
                created_at INTEGER,
                sort_index INTEGER,
                notes TEXT,
                meta TEXT,
                icon TEXT,
                icon_color TEXT,
                in_failover_queue INTEGER DEFAULT 0,
                is_current INTEGER DEFAULT 0,
                PRIMARY KEY (id, app_type)
            );

            -- MCP 服务器表
            CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                config TEXT NOT NULL,
                apps TEXT NOT NULL,
                enabled INTEGER DEFAULT 1,
                created_at INTEGER,
                sort_index INTEGER
            );

            -- 设置表
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- 创建索引
            CREATE INDEX IF NOT EXISTS idx_providers_app_type ON providers(app_type);
            CREATE INDEX IF NOT EXISTS idx_providers_is_current ON providers(is_current);
            "#,
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ===== Provider DAO =====

    /// 获取所有供应商
    pub fn get_all_providers(&self, app_type: &str) -> Result<IndexMap<String, Provider>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, name, settings_config, website_url, category, created_at,
                       sort_index, notes, meta, icon, icon_color, in_failover_queue
                FROM providers
                WHERE app_type = ?
                ORDER BY sort_index ASC, created_at ASC
                "#,
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let providers = stmt
            .query_map([app_type], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let settings_config_str: String = row.get(2)?;
                let website_url: Option<String> = row.get(3)?;
                let category: Option<String> = row.get(4)?;
                let created_at: Option<i64> = row.get(5)?;
                let sort_index: Option<usize> = row.get::<_, Option<i64>>(6)?.map(|v| v as usize);
                let notes: Option<String> = row.get(7)?;
                let meta_str: Option<String> = row.get(8)?;
                let icon: Option<String> = row.get(9)?;
                let icon_color: Option<String> = row.get(10)?;
                let in_failover_queue: bool = row.get::<_, i64>(11)? != 0;

                Ok((
                    id.clone(),
                    Provider {
                        id,
                        name,
                        settings_config: serde_json::from_str(&settings_config_str)
                            .unwrap_or_default(),
                        website_url,
                        category,
                        created_at,
                        sort_index,
                        notes,
                        meta: meta_str.and_then(|s| serde_json::from_str(&s).ok()),
                        icon,
                        icon_color,
                        in_failover_queue,
                    },
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut result = IndexMap::new();
        for provider_result in providers {
            let (id, provider) = provider_result.map_err(|e| AppError::Database(e.to_string()))?;
            result.insert(id, provider);
        }

        Ok(result)
    }

    /// 保存供应商
    pub fn save_provider(&self, app_type: &str, provider: &Provider) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        let settings_config_str = to_json_string(&provider.settings_config)?;
        let meta_str = provider
            .meta
            .as_ref()
            .map(|m| to_json_string(m))
            .transpose()?;

        conn.execute(
            r#"
            INSERT OR REPLACE INTO providers
            (id, app_type, name, settings_config, website_url, category, created_at,
             sort_index, notes, meta, icon, icon_color, in_failover_queue)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                provider.id,
                app_type,
                provider.name,
                settings_config_str,
                provider.website_url,
                provider.category,
                provider.created_at,
                provider.sort_index.map(|v| v as i64),
                provider.notes,
                meta_str,
                provider.icon,
                provider.icon_color,
                provider.in_failover_queue as i64,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 删除供应商
    pub fn delete_provider(&self, app_type: &str, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "DELETE FROM providers WHERE id = ? AND app_type = ?",
            rusqlite::params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 获取当前供应商 ID
    pub fn get_current_provider(&self, app_type: &str) -> Result<Option<String>, AppError> {
        let conn = lock_conn!(self.conn);

        let result: Option<String> = conn
            .query_row(
                "SELECT id FROM providers WHERE app_type = ? AND is_current = 1",
                [app_type],
                |row| row.get(0),
            )
            .ok();

        Ok(result)
    }

    /// 设置当前供应商
    pub fn set_current_provider(&self, app_type: &str, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        // 先清除所有 is_current
        conn.execute(
            "UPDATE providers SET is_current = 0 WHERE app_type = ?",
            [app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 设置新的当前供应商
        conn.execute(
            "UPDATE providers SET is_current = 1 WHERE id = ? AND app_type = ?",
            rusqlite::params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 获取供应商数量
    pub fn get_provider_count(&self, app_type: &str) -> Result<usize, AppError> {
        let conn = lock_conn!(self.conn);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM providers WHERE app_type = ?",
                [app_type],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as usize)
    }

    // ===== Settings DAO =====

    /// 获取设置值
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, AppError> {
        let conn = lock_conn!(self.conn);

        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?",
                [key],
                |row| row.get(0),
            )
            .ok();

        Ok(result)
    }

    /// 设置值
    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
            rusqlite::params![key, value],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 删除设置
    pub fn delete_setting(&self, key: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute("DELETE FROM settings WHERE key = ?", [key])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_database_init() {
        let db = Database::memory().unwrap();
        assert!(db.get_all_providers("claude").unwrap().is_empty());
    }

    #[test]
    fn test_provider_crud() {
        let db = Database::memory().unwrap();

        let provider = Provider::new("p1", "Provider 1", json!({"key": "value"}));
        db.save_provider("claude", &provider).unwrap();

        let providers = db.get_all_providers("claude").unwrap();
        assert_eq!(providers.len(), 1);
        assert!(providers.contains_key("p1"));

        db.delete_provider("claude", "p1").unwrap();
        let providers = db.get_all_providers("claude").unwrap();
        assert!(providers.is_empty());
    }

    #[test]
    fn test_current_provider() {
        let db = Database::memory().unwrap();

        let p1 = Provider::new("p1", "Provider 1", json!({}));
        let p2 = Provider::new("p2", "Provider 2", json!({}));

        db.save_provider("claude", &p1).unwrap();
        db.save_provider("claude", &p2).unwrap();

        db.set_current_provider("claude", "p1").unwrap();
        assert_eq!(
            db.get_current_provider("claude").unwrap(),
            Some("p1".to_string())
        );

        db.set_current_provider("claude", "p2").unwrap();
        assert_eq!(
            db.get_current_provider("claude").unwrap(),
            Some("p2".to_string())
        );
    }

    #[test]
    fn test_settings() {
        let db = Database::memory().unwrap();

        db.set_setting("key1", "value1").unwrap();
        assert_eq!(db.get_setting("key1").unwrap(), Some("value1".to_string()));

        db.delete_setting("key1").unwrap();
        assert_eq!(db.get_setting("key1").unwrap(), None);
    }
}
