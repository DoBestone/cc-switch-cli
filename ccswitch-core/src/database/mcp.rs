//! MCP 服务器数据库操作模块

use crate::app_config::McpApps;
use crate::database::{lock_conn, to_json_string, Database};
use crate::error::AppError;
use crate::mcp::McpServer;
use indexmap::IndexMap;

impl Database {
    // ===== MCP Server DAO =====

    /// 获取所有 MCP 服务器
    pub fn get_all_mcp_servers(&self) -> Result<IndexMap<String, McpServer>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, name, server_config, description, homepage, docs, tags,
                       enabled_claude, enabled_codex, enabled_gemini, enabled_opencode,
                       created_at, sort_index
                FROM mcp_servers
                ORDER BY sort_index ASC, created_at ASC
                "#,
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let servers = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let server_config_str: String = row.get(2)?;
                let description: Option<String> = row.get(3)?;
                let homepage: Option<String> = row.get(4)?;
                let docs: Option<String> = row.get(5)?;
                let tags_str: Option<String> = row.get(6)?;
                let enabled_claude: bool = row.get::<_, i64>(7)? != 0;
                let enabled_codex: bool = row.get::<_, i64>(8)? != 0;
                let enabled_gemini: bool = row.get::<_, i64>(9)? != 0;
                let enabled_opencode: bool = row.get::<_, i64>(10)? != 0;
                let created_at: Option<i64> = row.get(11)?;
                let sort_index: Option<usize> = row.get::<_, Option<i64>>(12)?.map(|v| v as usize);

                let server_config: serde_json::Value =
                    serde_json::from_str(&server_config_str).unwrap_or_default();
                let tags: Vec<String> = tags_str
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                Ok((
                    id.clone(),
                    McpServer {
                        id,
                        name,
                        server_config,
                        apps: McpApps {
                            claude: enabled_claude,
                            codex: enabled_codex,
                            gemini: enabled_gemini,
                            opencode: enabled_opencode,
                        },
                        description,
                        homepage,
                        docs,
                        tags,
                        created_at,
                        sort_index,
                    },
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut result = IndexMap::new();
        for server_result in servers {
            let (id, server) = server_result.map_err(|e| AppError::Database(e.to_string()))?;
            result.insert(id, server);
        }

        Ok(result)
    }

    /// 获取单个 MCP 服务器
    pub fn get_mcp_server(&self, id: &str) -> Result<Option<McpServer>, AppError> {
        let servers = self.get_all_mcp_servers()?;
        Ok(servers.get(id).cloned())
    }

    /// 保存 MCP 服务器
    pub fn save_mcp_server(&self, server: &McpServer) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        let server_config_str = to_json_string(&server.server_config)?;
        let tags_str = to_json_string(&server.tags)?;

        conn.execute(
            r#"
            INSERT OR REPLACE INTO mcp_servers
            (id, name, server_config, description, homepage, docs, tags,
             enabled_claude, enabled_codex, enabled_gemini, enabled_opencode,
             created_at, sort_index)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                server.id,
                server.name,
                server_config_str,
                server.description,
                server.homepage,
                server.docs,
                tags_str,
                server.apps.claude as i64,
                server.apps.codex as i64,
                server.apps.gemini as i64,
                server.apps.opencode as i64,
                server.created_at,
                server.sort_index.map(|v| v as i64),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 删除 MCP 服务器
    pub fn delete_mcp_server(&self, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute("DELETE FROM mcp_servers WHERE id = ?", rusqlite::params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// 更新 MCP 服务器的应用启用状态
    pub fn update_mcp_server_apps(&self, id: &str, apps: &McpApps) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            r#"
            UPDATE mcp_servers
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

    /// 获取 MCP 服务器数量
    pub fn get_mcp_server_count(&self) -> Result<usize, AppError> {
        let conn = lock_conn!(self.conn);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mcp_servers", [], |row| row.get(0))
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_server_crud() {
        let db = Database::memory().unwrap();

        let server = McpServer::new("test-server", "Test Server", json!({"command": "npx"}));
        db.save_mcp_server(&server).unwrap();

        let servers = db.get_all_mcp_servers().unwrap();
        assert_eq!(servers.len(), 1);
        assert!(servers.contains_key("test-server"));

        db.delete_mcp_server("test-server").unwrap();
        let servers = db.get_all_mcp_servers().unwrap();
        assert!(servers.is_empty());
    }

    #[test]
    fn test_mcp_server_apps_update() {
        let db = Database::memory().unwrap();

        let server = McpServer::new("test-server", "Test Server", json!({"command": "npx"}));
        db.save_mcp_server(&server).unwrap();

        let apps = McpApps {
            claude: true,
            codex: false,
            gemini: true,
            opencode: false,
        };
        db.update_mcp_server_apps("test-server", &apps).unwrap();

        let server = db.get_mcp_server("test-server").unwrap().unwrap();
        assert!(server.apps.claude);
        assert!(!server.apps.codex);
        assert!(server.apps.gemini);
        assert!(!server.apps.opencode);
    }
}

