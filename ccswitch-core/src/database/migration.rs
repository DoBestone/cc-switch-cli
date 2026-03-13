//! JSON → SQLite 数据迁移
//!
//! 将旧版 config.json 数据迁移到 SQLite 数据库。
//!
//! CLI-pro 不依赖 Tauri，迁移逻辑直接读取配置目录下的 JSON 文件。

use super::{lock_conn, to_json_string, Database};
use crate::error::AppError;
use rusqlite::{params, Connection};

impl Database {
    /// 执行 dry-run 模式（在内存数据库中验证，不写入磁盘）
    ///
    /// 用于部署前验证迁移逻辑是否正确。
    pub fn migrate_dry_run() -> Result<(), AppError> {
        let conn =
            Connection::open_in_memory().map_err(|e| AppError::Database(e.to_string()))?;
        Self::create_tables_on_conn(&conn)?;
        Self::apply_schema_migrations_on_conn(&conn)?;
        Ok(())
    }

    /// 从 JSON 配置目录迁移供应商数据
    ///
    /// 读取指定目录下的 providers.json（格式为 `{ "id": {...Provider} }` 的 JSON 对象），
    /// 将其写入数据库。如果文件不存在则跳过。
    pub fn migrate_providers_from_json_file(
        &self,
        json_path: &std::path::Path,
        app_type: &str,
    ) -> Result<usize, AppError> {
        if !json_path.exists() {
            return Ok(0);
        }

        let content = std::fs::read_to_string(json_path)
            .map_err(|e| AppError::io(json_path, e))?;

        let providers_map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&content).map_err(|e| AppError::json(json_path, e))?;

        let mut conn = lock_conn!(self.conn);
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut count = 0;
        for (id, value) in &providers_map {
            let name = value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(id.as_str())
                .to_string();

            let settings_config = value
                .get("settingsConfig")
                .or_else(|| value.get("settings_config"))
                .cloned()
                .unwrap_or(serde_json::json!({}));

            let settings_config_str = to_json_string(&settings_config)?;
            let website_url = value
                .get("websiteUrl")
                .or_else(|| value.get("website_url"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let category = value
                .get("category")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let created_at = value
                .get("createdAt")
                .or_else(|| value.get("created_at"))
                .and_then(|v| v.as_i64());
            let sort_index = value
                .get("sortIndex")
                .or_else(|| value.get("sort_index"))
                .and_then(|v| v.as_i64());
            let notes = value
                .get("notes")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let icon = value
                .get("icon")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let icon_color = value
                .get("iconColor")
                .or_else(|| value.get("icon_color"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let meta_str = value
                .get("meta")
                .map(|m| to_json_string(m))
                .transpose()?;
            let in_failover_queue = value
                .get("inFailoverQueue")
                .or_else(|| value.get("in_failover_queue"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            tx.execute(
                "INSERT OR REPLACE INTO providers (
                    id, app_type, name, settings_config, website_url, category,
                    created_at, sort_index, notes, icon, icon_color, meta, in_failover_queue
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    id,
                    app_type,
                    name,
                    settings_config_str,
                    website_url,
                    category,
                    created_at,
                    sort_index,
                    notes,
                    icon,
                    icon_color,
                    meta_str.unwrap_or_else(|| "{}".to_string()),
                    in_failover_queue as i64,
                ],
            )
            .map_err(|e| AppError::Database(format!("迁移供应商 {id} 失败: {e}")))?;

            // 迁移 custom_endpoints（如果存在）
            if let Some(meta) = value.get("meta") {
                if let Some(endpoints) = meta
                    .get("customEndpoints")
                    .or_else(|| meta.get("custom_endpoints"))
                    .and_then(|v| v.as_object())
                {
                    for (url, endpoint_val) in endpoints {
                        let added_at = endpoint_val
                            .get("addedAt")
                            .or_else(|| endpoint_val.get("added_at"))
                            .and_then(|v| v.as_i64());
                        let _ = tx.execute(
                            "INSERT OR IGNORE INTO provider_endpoints (provider_id, app_type, url, added_at)
                             VALUES (?1, ?2, ?3, ?4)",
                            params![id, app_type, url, added_at],
                        );
                    }
                }
            }

            count += 1;
        }

        tx.commit()
            .map_err(|e| AppError::Database(format!("提交供应商迁移失败: {e}")))?;

        log::info!("从 {} 迁移了 {count} 个 {app_type} 供应商", json_path.display());
        Ok(count)
    }

    /// 从设置 JSON 文件迁移设置键值对
    ///
    /// 读取 JSON 对象并将其写入 settings 表。
    pub fn migrate_settings_from_json_file(
        &self,
        json_path: &std::path::Path,
    ) -> Result<usize, AppError> {
        if !json_path.exists() {
            return Ok(0);
        }

        let content = std::fs::read_to_string(json_path)
            .map_err(|e| AppError::io(json_path, e))?;

        let settings: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&content).map_err(|e| AppError::json(json_path, e))?;

        let conn = lock_conn!(self.conn);
        let mut count = 0;

        for (key, value) in &settings {
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                other => to_json_string(other)?,
            };
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value_str],
            )
            .map_err(|e| AppError::Database(format!("迁移设置 {key} 失败: {e}")))?;
            count += 1;
        }

        log::info!("从 {} 迁移了 {count} 条设置", json_path.display());
        Ok(count)
    }
}
