//! Provider Endpoints DAO
//!
//! 管理供应商的自定义 Endpoint URL 列表（存储在 provider_endpoints 表）

use crate::database::{lock_conn, Database};
use crate::error::AppError;

#[allow(dead_code)]
impl Database {
    /// 添加 provider endpoint URL
    pub(crate) fn add_provider_endpoint(
        &self,
        provider_id: &str,
        app_type: &str,
        url: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO provider_endpoints (provider_id, app_type, url, added_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![provider_id, app_type, url, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 获取指定供应商和应用类型的所有 endpoint URL
    pub(crate) fn get_provider_endpoints(
        &self,
        provider_id: &str,
        app_type: &str,
    ) -> Result<Vec<String>, AppError> {
        let conn = lock_conn!(self.conn);

        let mut stmt = conn
            .prepare(
                "SELECT url FROM provider_endpoints
                 WHERE provider_id = ?1 AND app_type = ?2
                 ORDER BY added_at ASC, id ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let urls = stmt
            .query_map(rusqlite::params![provider_id, app_type], |row| {
                row.get(0)
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(urls)
    }

    /// 删除指定供应商和应用类型的某条 endpoint URL
    pub(crate) fn remove_provider_endpoint(
        &self,
        provider_id: &str,
        app_type: &str,
        url: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "DELETE FROM provider_endpoints
             WHERE provider_id = ?1 AND app_type = ?2 AND url = ?3",
            rusqlite::params![provider_id, app_type, url],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 清除指定供应商和应用类型的所有 endpoint URL
    pub(crate) fn clear_provider_endpoints(
        &self,
        provider_id: &str,
        app_type: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "DELETE FROM provider_endpoints WHERE provider_id = ?1 AND app_type = ?2",
            rusqlite::params![provider_id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use serde_json::json;

    #[test]
    fn test_provider_endpoints_crud() {
        let db = Database::memory().unwrap();

        let provider = crate::provider::Provider::new("p1", "Provider 1", json!({}));
        db.save_provider("claude", &provider).unwrap();

        db.add_provider_endpoint("p1", "claude", "https://api.example.com").unwrap();
        db.add_provider_endpoint("p1", "claude", "https://api2.example.com").unwrap();

        let urls = db.get_provider_endpoints("p1", "claude").unwrap();
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://api.example.com".to_string()));

        db.remove_provider_endpoint("p1", "claude", "https://api.example.com").unwrap();
        let urls = db.get_provider_endpoints("p1", "claude").unwrap();
        assert_eq!(urls.len(), 1);

        db.clear_provider_endpoints("p1", "claude").unwrap();
        let urls = db.get_provider_endpoints("p1", "claude").unwrap();
        assert!(urls.is_empty());
    }
}
