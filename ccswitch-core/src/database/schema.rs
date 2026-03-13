//! Schema 定义和迁移
//!
//! 负责数据库表结构的创建和版本迁移。

use super::{lock_conn, Database, SCHEMA_VERSION};
use crate::error::AppError;
use rusqlite::Connection;

impl Database {
    /// 创建所有数据库表
    pub(crate) fn create_tables(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::create_tables_on_conn(&conn)
    }

    /// 在指定连接上创建表（供迁移和测试使用）
    pub(crate) fn create_tables_on_conn(conn: &Connection) -> Result<(), AppError> {
        // 1. Providers 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS providers (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                settings_config TEXT NOT NULL,
                website_url TEXT,
                category TEXT,
                created_at INTEGER,
                sort_index INTEGER,
                notes TEXT,
                meta TEXT NOT NULL DEFAULT '{}',
                icon TEXT,
                icon_color TEXT,
                in_failover_queue BOOLEAN NOT NULL DEFAULT 0,
                is_current BOOLEAN NOT NULL DEFAULT 0,
                PRIMARY KEY (id, app_type)
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 2. Provider Endpoints 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS provider_endpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                url TEXT NOT NULL,
                added_at INTEGER,
                FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 3. MCP Servers 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mcp_servers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                server_config TEXT NOT NULL,
                description TEXT,
                homepage TEXT,
                docs TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                enabled_claude BOOLEAN NOT NULL DEFAULT 0,
                enabled_codex BOOLEAN NOT NULL DEFAULT 0,
                enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
                enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
                created_at INTEGER,
                sort_index INTEGER
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 4. Prompts 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS prompts (
                id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                name TEXT NOT NULL,
                content TEXT NOT NULL,
                description TEXT,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at INTEGER,
                updated_at INTEGER,
                PRIMARY KEY (id, app_type)
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 5. Skills 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                directory TEXT NOT NULL,
                repo_owner TEXT,
                repo_name TEXT,
                repo_branch TEXT DEFAULT 'main',
                readme_url TEXT,
                enabled_claude BOOLEAN NOT NULL DEFAULT 0,
                enabled_codex BOOLEAN NOT NULL DEFAULT 0,
                enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
                enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
                installed_at INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 6. Skill Repos 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS skill_repos (
                id TEXT PRIMARY KEY,
                owner TEXT NOT NULL,
                name TEXT NOT NULL,
                branch TEXT DEFAULT 'main',
                enabled INTEGER DEFAULT 1
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 7. Settings 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 8. Proxy Config 表（per app_type）
        conn.execute(
            "CREATE TABLE IF NOT EXISTS proxy_config (
                app_type TEXT PRIMARY KEY CHECK (app_type IN ('claude','codex','gemini')),
                proxy_enabled INTEGER NOT NULL DEFAULT 0,
                listen_address TEXT NOT NULL DEFAULT '127.0.0.1',
                listen_port INTEGER NOT NULL DEFAULT 15721,
                enable_logging INTEGER NOT NULL DEFAULT 1,
                enabled INTEGER NOT NULL DEFAULT 0,
                auto_failover_enabled INTEGER NOT NULL DEFAULT 0,
                max_retries INTEGER NOT NULL DEFAULT 3,
                streaming_first_byte_timeout INTEGER NOT NULL DEFAULT 60,
                streaming_idle_timeout INTEGER NOT NULL DEFAULT 120,
                non_streaming_timeout INTEGER NOT NULL DEFAULT 600,
                circuit_failure_threshold INTEGER NOT NULL DEFAULT 4,
                circuit_success_threshold INTEGER NOT NULL DEFAULT 2,
                circuit_timeout_seconds INTEGER NOT NULL DEFAULT 60,
                circuit_error_rate_threshold REAL NOT NULL DEFAULT 0.6,
                circuit_min_requests INTEGER NOT NULL DEFAULT 10,
                default_cost_multiplier TEXT NOT NULL DEFAULT '1',
                pricing_model_source TEXT NOT NULL DEFAULT 'response',
                live_takeover_active INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Seed three rows for proxy_config if the table has the app_type column
        if Self::has_column(conn, "proxy_config", "app_type")? {
            conn.execute(
                "INSERT OR IGNORE INTO proxy_config (app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests)
                VALUES ('claude', 6, 90, 180, 600, 8, 3, 90, 0.7, 15)",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "INSERT OR IGNORE INTO proxy_config (app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests)
                VALUES ('codex', 3, 60, 120, 600, 4, 2, 60, 0.6, 10)",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "INSERT OR IGNORE INTO proxy_config (app_type, max_retries,
                streaming_first_byte_timeout, streaming_idle_timeout, non_streaming_timeout,
                circuit_failure_threshold, circuit_success_threshold, circuit_timeout_seconds,
                circuit_error_rate_threshold, circuit_min_requests)
                VALUES ('gemini', 5, 60, 120, 600, 4, 2, 60, 0.6, 10)",
                [],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        // 9. Provider Health 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS provider_health (
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                is_healthy INTEGER NOT NULL DEFAULT 1,
                consecutive_failures INTEGER NOT NULL DEFAULT 0,
                last_success_at TEXT,
                last_failure_at TEXT,
                last_error TEXT,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (provider_id, app_type),
                FOREIGN KEY (provider_id, app_type) REFERENCES providers(id, app_type) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 10. Proxy Request Logs 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS proxy_request_logs (
                request_id TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL,
                app_type TEXT NOT NULL,
                model TEXT NOT NULL,
                request_model TEXT,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                input_cost_usd TEXT NOT NULL DEFAULT '0',
                output_cost_usd TEXT NOT NULL DEFAULT '0',
                cache_read_cost_usd TEXT NOT NULL DEFAULT '0',
                cache_creation_cost_usd TEXT NOT NULL DEFAULT '0',
                total_cost_usd TEXT NOT NULL DEFAULT '0',
                latency_ms INTEGER NOT NULL,
                first_token_ms INTEGER,
                duration_ms INTEGER,
                status_code INTEGER NOT NULL,
                error_message TEXT,
                session_id TEXT,
                provider_type TEXT,
                is_streaming INTEGER NOT NULL DEFAULT 0,
                cost_multiplier TEXT NOT NULL DEFAULT '1.0',
                created_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_provider ON proxy_request_logs(provider_id, app_type)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON proxy_request_logs(created_at)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_model ON proxy_request_logs(model)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_session ON proxy_request_logs(session_id)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_request_logs_status ON proxy_request_logs(status_code)",
            [],
        );

        // 11. Model Pricing 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS model_pricing (
                model_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                input_cost_per_million TEXT NOT NULL,
                output_cost_per_million TEXT NOT NULL,
                cache_read_cost_per_million TEXT NOT NULL DEFAULT '0',
                cache_creation_cost_per_million TEXT NOT NULL DEFAULT '0'
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 12. Stream Check Logs 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS stream_check_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                provider_name TEXT NOT NULL,
                app_type TEXT NOT NULL,
                status TEXT NOT NULL,
                success INTEGER NOT NULL,
                message TEXT NOT NULL,
                response_time_ms INTEGER,
                http_status INTEGER,
                model_used TEXT,
                retry_count INTEGER DEFAULT 0,
                tested_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_stream_check_logs_provider
             ON stream_check_logs(app_type, provider_id, tested_at DESC)",
            [],
        );

        // 13. Proxy Live Backup 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS proxy_live_backup (
                app_type TEXT PRIMARY KEY,
                original_config TEXT NOT NULL,
                backed_up_at TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // 14. Usage Daily Rollups 表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS usage_daily_rollups (
                date TEXT NOT NULL,
                app_type TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                model TEXT NOT NULL,
                request_count INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                total_cost_usd TEXT NOT NULL DEFAULT '0',
                avg_latency_ms INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (date, app_type, provider_id, model)
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Indexes for providers
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_providers_app_type ON providers(app_type)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_providers_is_current ON providers(is_current)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_providers_failover
             ON providers(app_type, in_failover_queue, sort_index)",
            [],
        );

        // Ensure in_failover_queue column exists on older databases
        Self::add_column_if_missing(
            conn,
            "providers",
            "in_failover_queue",
            "BOOLEAN NOT NULL DEFAULT 0",
        )?;

        // Drop old failover_queue table if it exists
        let _ = conn.execute("DROP INDEX IF EXISTS idx_failover_queue_order", []);
        let _ = conn.execute("DROP TABLE IF EXISTS failover_queue", []);

        Ok(())
    }

    /// 应用 Schema 迁移
    pub(crate) fn apply_schema_migrations(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::apply_schema_migrations_on_conn(&conn)
    }

    /// 在指定连接上应用 Schema 迁移
    pub(crate) fn apply_schema_migrations_on_conn(conn: &Connection) -> Result<(), AppError> {
        conn.execute("SAVEPOINT schema_migration;", [])
            .map_err(|e| AppError::Database(format!("开启迁移 savepoint 失败: {e}")))?;

        let mut version = Self::get_user_version(conn)?;

        if version > SCHEMA_VERSION {
            conn.execute("ROLLBACK TO schema_migration;", []).ok();
            conn.execute("RELEASE schema_migration;", []).ok();
            return Err(AppError::Database(format!(
                "数据库版本过新（{version}），当前应用仅支持 {SCHEMA_VERSION}，请升级应用后再尝试。"
            )));
        }

        let result = (|| {
            while version < SCHEMA_VERSION {
                match version {
                    0 => {
                        log::info!("检测到 user_version=0，迁移到 1（补齐缺失列并设置版本）");
                        Self::migrate_v0_to_v1(conn)?;
                        Self::set_user_version(conn, 1)?;
                    }
                    1 => {
                        log::info!("迁移数据库从 v1 到 v2（添加使用统计表和完整字段）");
                        Self::migrate_v1_to_v2(conn)?;
                        Self::set_user_version(conn, 2)?;
                    }
                    2 => {
                        log::info!("迁移数据库从 v2 到 v3（Skills 统一管理架构）");
                        Self::migrate_v2_to_v3(conn)?;
                        Self::set_user_version(conn, 3)?;
                    }
                    3 => {
                        log::info!("迁移数据库从 v3 到 v4（OpenCode 支持）");
                        Self::migrate_v3_to_v4(conn)?;
                        Self::set_user_version(conn, 4)?;
                    }
                    4 => {
                        log::info!("迁移数据库从 v4 到 v5（计费模式支持）");
                        Self::migrate_v4_to_v5(conn)?;
                        Self::set_user_version(conn, 5)?;
                    }
                    5 => {
                        log::info!("迁移数据库从 v5 到 v6（使用量聚合表）");
                        Self::migrate_v5_to_v6(conn)?;
                        Self::set_user_version(conn, 6)?;
                    }
                    _ => {
                        return Err(AppError::Database(format!(
                            "未知的数据库版本 {version}，无法迁移到 {SCHEMA_VERSION}"
                        )));
                    }
                }
                version = Self::get_user_version(conn)?;
            }
            Ok(())
        })();

        match result {
            Ok(_) => {
                conn.execute("RELEASE schema_migration;", [])
                    .map_err(|e| AppError::Database(format!("提交迁移 savepoint 失败: {e}")))?;
                Ok(())
            }
            Err(e) => {
                conn.execute("ROLLBACK TO schema_migration;", []).ok();
                conn.execute("RELEASE schema_migration;", []).ok();
                Err(e)
            }
        }
    }

    /// v0 -> v1: 补齐缺失列
    fn migrate_v0_to_v1(conn: &Connection) -> Result<(), AppError> {
        Self::add_column_if_missing(conn, "providers", "category", "TEXT")?;
        Self::add_column_if_missing(conn, "providers", "created_at", "INTEGER")?;
        Self::add_column_if_missing(conn, "providers", "sort_index", "INTEGER")?;
        Self::add_column_if_missing(conn, "providers", "notes", "TEXT")?;
        Self::add_column_if_missing(conn, "providers", "icon", "TEXT")?;
        Self::add_column_if_missing(conn, "providers", "icon_color", "TEXT")?;
        Self::add_column_if_missing(conn, "providers", "meta", "TEXT NOT NULL DEFAULT '{}'")?;
        Self::add_column_if_missing(conn, "providers", "is_current", "BOOLEAN NOT NULL DEFAULT 0")?;
        Self::add_column_if_missing(conn, "providers", "in_failover_queue", "BOOLEAN NOT NULL DEFAULT 0")?;

        if Self::table_exists(conn, "provider_endpoints")? {
            Self::add_column_if_missing(conn, "provider_endpoints", "added_at", "INTEGER")?;
        }

        Self::add_column_if_missing(conn, "mcp_servers", "description", "TEXT")?;
        Self::add_column_if_missing(conn, "mcp_servers", "homepage", "TEXT")?;
        Self::add_column_if_missing(conn, "mcp_servers", "docs", "TEXT")?;
        Self::add_column_if_missing(conn, "mcp_servers", "tags", "TEXT NOT NULL DEFAULT '[]'")?;
        Self::add_column_if_missing(conn, "mcp_servers", "enabled_codex", "BOOLEAN NOT NULL DEFAULT 0")?;
        Self::add_column_if_missing(conn, "mcp_servers", "enabled_gemini", "BOOLEAN NOT NULL DEFAULT 0")?;
        Self::add_column_if_missing(conn, "mcp_servers", "created_at", "INTEGER")?;
        Self::add_column_if_missing(conn, "mcp_servers", "sort_index", "INTEGER")?;

        Self::add_column_if_missing(conn, "prompts", "description", "TEXT")?;
        Self::add_column_if_missing(conn, "prompts", "enabled", "BOOLEAN NOT NULL DEFAULT 1")?;
        Self::add_column_if_missing(conn, "prompts", "created_at", "INTEGER")?;
        Self::add_column_if_missing(conn, "prompts", "updated_at", "INTEGER")?;

        if Self::table_exists(conn, "skills")? {
            Self::add_column_if_missing(conn, "skills", "installed_at", "INTEGER NOT NULL DEFAULT 0")?;
        }

        if Self::table_exists(conn, "skill_repos")? {
            Self::add_column_if_missing(conn, "skill_repos", "branch", "TEXT NOT NULL DEFAULT 'main'")?;
            Self::add_column_if_missing(conn, "skill_repos", "enabled", "BOOLEAN NOT NULL DEFAULT 1")?;
        }

        Ok(())
    }

    /// v1 -> v2: 添加使用统计表和完整字段
    fn migrate_v1_to_v2(conn: &Connection) -> Result<(), AppError> {
        Self::add_column_if_missing(conn, "providers", "in_failover_queue", "BOOLEAN NOT NULL DEFAULT 0")?;

        if Self::table_exists(conn, "proxy_config")? {
            Self::add_column_if_missing(conn, "proxy_config", "proxy_enabled", "INTEGER NOT NULL DEFAULT 0")?;
            Self::add_column_if_missing(conn, "proxy_config", "listen_address", "TEXT NOT NULL DEFAULT '127.0.0.1'")?;
            Self::add_column_if_missing(conn, "proxy_config", "listen_port", "INTEGER NOT NULL DEFAULT 15721")?;
            Self::add_column_if_missing(conn, "proxy_config", "enable_logging", "INTEGER NOT NULL DEFAULT 1")?;
            Self::add_column_if_missing(conn, "proxy_config", "streaming_first_byte_timeout", "INTEGER NOT NULL DEFAULT 60")?;
            Self::add_column_if_missing(conn, "proxy_config", "streaming_idle_timeout", "INTEGER NOT NULL DEFAULT 120")?;
            Self::add_column_if_missing(conn, "proxy_config", "non_streaming_timeout", "INTEGER NOT NULL DEFAULT 600")?;
        }

        let _ = conn.execute("DROP INDEX IF EXISTS idx_failover_queue_order", []);
        let _ = conn.execute("DROP TABLE IF EXISTS failover_queue", []);
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_providers_failover
             ON providers(app_type, in_failover_queue, sort_index)",
            [],
        );

        conn.execute(
            "CREATE TABLE IF NOT EXISTS proxy_request_logs (
                request_id TEXT PRIMARY KEY, provider_id TEXT NOT NULL, app_type TEXT NOT NULL, model TEXT NOT NULL,
                request_model TEXT,
                input_tokens INTEGER NOT NULL DEFAULT 0, output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0, cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                input_cost_usd TEXT NOT NULL DEFAULT '0', output_cost_usd TEXT NOT NULL DEFAULT '0',
                cache_read_cost_usd TEXT NOT NULL DEFAULT '0', cache_creation_cost_usd TEXT NOT NULL DEFAULT '0',
                total_cost_usd TEXT NOT NULL DEFAULT '0', latency_ms INTEGER NOT NULL, first_token_ms INTEGER,
                duration_ms INTEGER, status_code INTEGER NOT NULL, error_message TEXT, session_id TEXT,
                provider_type TEXT, is_streaming INTEGER NOT NULL DEFAULT 0,
                cost_multiplier TEXT NOT NULL DEFAULT '1.0', created_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        if Self::table_exists(conn, "proxy_request_logs")? {
            Self::add_column_if_missing(conn, "proxy_request_logs", "provider_type", "TEXT")?;
            Self::add_column_if_missing(conn, "proxy_request_logs", "is_streaming", "INTEGER NOT NULL DEFAULT 0")?;
            Self::add_column_if_missing(conn, "proxy_request_logs", "cost_multiplier", "TEXT NOT NULL DEFAULT '1.0'")?;
            Self::add_column_if_missing(conn, "proxy_request_logs", "first_token_ms", "INTEGER")?;
            Self::add_column_if_missing(conn, "proxy_request_logs", "duration_ms", "INTEGER")?;
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS model_pricing (
                model_id TEXT PRIMARY KEY, display_name TEXT NOT NULL,
                input_cost_per_million TEXT NOT NULL, output_cost_per_million TEXT NOT NULL,
                cache_read_cost_per_million TEXT NOT NULL DEFAULT '0',
                cache_creation_cost_per_million TEXT NOT NULL DEFAULT '0'
            )",
            [],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        conn.execute("DELETE FROM model_pricing", [])
            .map_err(|e| AppError::Database(format!("清空模型定价失败: {e}")))?;
        Self::seed_model_pricing(conn)?;

        Ok(())
    }

    /// v2 -> v3: Skills 统一管理架构
    fn migrate_v2_to_v3(conn: &Connection) -> Result<(), AppError> {
        if Self::has_column(conn, "skills", "enabled_claude")? {
            log::info!("skills 表已经是 v3 结构，跳过迁移");
            return Ok(());
        }

        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('skills_ssot_migration_pending', 'true')",
            [],
        );

        let _ = conn.execute("DROP TABLE IF EXISTS skills", []);

        conn.execute(
            "CREATE TABLE skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                directory TEXT NOT NULL,
                repo_owner TEXT,
                repo_name TEXT,
                repo_branch TEXT DEFAULT 'main',
                readme_url TEXT,
                enabled_claude BOOLEAN NOT NULL DEFAULT 0,
                enabled_codex BOOLEAN NOT NULL DEFAULT 0,
                enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
                installed_at INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建新 skills 表失败: {e}")))?;

        log::info!("skills 表已迁移到 v3 结构");
        Ok(())
    }

    /// v3 -> v4: 添加 OpenCode 支持
    fn migrate_v3_to_v4(conn: &Connection) -> Result<(), AppError> {
        Self::add_column_if_missing(conn, "mcp_servers", "enabled_opencode", "BOOLEAN NOT NULL DEFAULT 0")?;
        Self::add_column_if_missing(conn, "skills", "enabled_opencode", "BOOLEAN NOT NULL DEFAULT 0")?;
        log::info!("v3 -> v4 迁移完成：已添加 OpenCode 支持");
        Ok(())
    }

    /// v4 -> v5: 新增计费模式配置与请求模型字段
    fn migrate_v4_to_v5(conn: &Connection) -> Result<(), AppError> {
        if Self::table_exists(conn, "proxy_config")? {
            Self::add_column_if_missing(conn, "proxy_config", "default_cost_multiplier", "TEXT NOT NULL DEFAULT '1'")?;
            Self::add_column_if_missing(conn, "proxy_config", "pricing_model_source", "TEXT NOT NULL DEFAULT 'response'")?;
        }
        if Self::table_exists(conn, "proxy_request_logs")? {
            Self::add_column_if_missing(conn, "proxy_request_logs", "request_model", "TEXT")?;
        }
        log::info!("v4 -> v5 迁移完成：已添加计费模式与请求模型字段");
        Ok(())
    }

    /// v5 -> v6: 添加使用量日聚合表
    fn migrate_v5_to_v6(conn: &Connection) -> Result<(), AppError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS usage_daily_rollups (
                date TEXT NOT NULL,
                app_type TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                model TEXT NOT NULL,
                request_count INTEGER NOT NULL DEFAULT 0,
                success_count INTEGER NOT NULL DEFAULT 0,
                input_tokens INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read_tokens INTEGER NOT NULL DEFAULT 0,
                cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
                total_cost_usd TEXT NOT NULL DEFAULT '0',
                avg_latency_ms INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (date, app_type, provider_id, model)
            )",
            [],
        )
        .map_err(|e| AppError::Database(format!("创建 usage_daily_rollups 表失败: {e}")))?;
        log::info!("v5 -> v6 迁移完成：已添加使用量日聚合表");
        Ok(())
    }

    /// 插入默认模型定价数据
    fn seed_model_pricing(conn: &Connection) -> Result<(), AppError> {
        let pricing_data: &[(&str, &str, &str, &str, &str, &str)] = &[
            // Claude 4.6 系列
            ("claude-opus-4-6-20260206", "Claude Opus 4.6", "5", "25", "0.50", "6.25"),
            // Claude 4.5 系列
            ("claude-opus-4-5-20251101", "Claude Opus 4.5", "5", "25", "0.50", "6.25"),
            ("claude-sonnet-4-5-20250929", "Claude Sonnet 4.5", "3", "15", "0.30", "3.75"),
            ("claude-haiku-4-5-20251001", "Claude Haiku 4.5", "1", "5", "0.10", "1.25"),
            // Claude 4 系列
            ("claude-opus-4-20250514", "Claude Opus 4", "15", "75", "1.50", "18.75"),
            ("claude-opus-4-1-20250805", "Claude Opus 4.1", "15", "75", "1.50", "18.75"),
            ("claude-sonnet-4-20250514", "Claude Sonnet 4", "3", "15", "0.30", "3.75"),
            // Claude 3.5 系列
            ("claude-3-5-haiku-20241022", "Claude 3.5 Haiku", "0.80", "4", "0.08", "1"),
            ("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet", "3", "15", "0.30", "3.75"),
            // GPT-5.2 系列
            ("gpt-5.2", "GPT-5.2", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-low", "GPT-5.2", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-medium", "GPT-5.2", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-high", "GPT-5.2", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-xhigh", "GPT-5.2", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-codex", "GPT-5.2 Codex", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-codex-low", "GPT-5.2 Codex", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-codex-medium", "GPT-5.2 Codex", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-codex-high", "GPT-5.2 Codex", "1.75", "14", "0.175", "0"),
            ("gpt-5.2-codex-xhigh", "GPT-5.2 Codex", "1.75", "14", "0.175", "0"),
            // GPT-5.3 Codex 系列
            ("gpt-5.3-codex", "GPT-5.3 Codex", "1.75", "14", "0.175", "0"),
            ("gpt-5.3-codex-low", "GPT-5.3 Codex", "1.75", "14", "0.175", "0"),
            ("gpt-5.3-codex-medium", "GPT-5.3 Codex", "1.75", "14", "0.175", "0"),
            ("gpt-5.3-codex-high", "GPT-5.3 Codex", "1.75", "14", "0.175", "0"),
            ("gpt-5.3-codex-xhigh", "GPT-5.3 Codex", "1.75", "14", "0.175", "0"),
            // GPT-5.1 系列
            ("gpt-5.1", "GPT-5.1", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-low", "GPT-5.1", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-medium", "GPT-5.1", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-high", "GPT-5.1", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-minimal", "GPT-5.1", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-codex", "GPT-5.1 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-codex-mini", "GPT-5.1 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-codex-max", "GPT-5.1 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-codex-max-high", "GPT-5.1 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5.1-codex-max-xhigh", "GPT-5.1 Codex", "1.25", "10", "0.125", "0"),
            // GPT-5 系列
            ("gpt-5", "GPT-5", "1.25", "10", "0.125", "0"),
            ("gpt-5-low", "GPT-5", "1.25", "10", "0.125", "0"),
            ("gpt-5-medium", "GPT-5", "1.25", "10", "0.125", "0"),
            ("gpt-5-high", "GPT-5", "1.25", "10", "0.125", "0"),
            ("gpt-5-minimal", "GPT-5", "1.25", "10", "0.125", "0"),
            ("gpt-5-codex", "GPT-5 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5-codex-low", "GPT-5 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5-codex-medium", "GPT-5 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5-codex-high", "GPT-5 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5-codex-mini", "GPT-5 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5-codex-mini-medium", "GPT-5 Codex", "1.25", "10", "0.125", "0"),
            ("gpt-5-codex-mini-high", "GPT-5 Codex", "1.25", "10", "0.125", "0"),
            // Gemini 3 系列
            ("gemini-3-pro-preview", "Gemini 3 Pro Preview", "2", "12", "0.2", "0"),
            ("gemini-3-flash-preview", "Gemini 3 Flash Preview", "0.5", "3", "0.05", "0"),
            // Gemini 2.5 系列
            ("gemini-2.5-pro", "Gemini 2.5 Pro", "1.25", "10", "0.125", "0"),
            ("gemini-2.5-flash", "Gemini 2.5 Flash", "0.3", "2.5", "0.03", "0"),
            // StepFun 系列
            ("step-3.5-flash", "Step 3.5 Flash", "0.10", "0.30", "0.02", "0"),
            // 国产模型
            ("doubao-seed-code", "Doubao Seed Code", "1.20", "8.00", "0.24", "0"),
            ("deepseek-v3.2", "DeepSeek V3.2", "2.00", "3.00", "0.40", "0"),
            ("deepseek-v3.1", "DeepSeek V3.1", "4.00", "12.00", "0.80", "0"),
            ("deepseek-v3", "DeepSeek V3", "2.00", "8.00", "0.40", "0"),
            ("kimi-k2-thinking", "Kimi K2 Thinking", "4.00", "16.00", "1.00", "0"),
            ("kimi-k2-0905", "Kimi K2", "4.00", "16.00", "1.00", "0"),
            ("kimi-k2-turbo", "Kimi K2 Turbo", "8.00", "58.00", "1.00", "0"),
            ("minimax-m2.1", "MiniMax M2.1", "2.10", "8.40", "0.21", "0"),
            ("minimax-m2.1-lightning", "MiniMax M2.1 Lightning", "2.10", "16.80", "0.21", "0"),
            ("minimax-m2", "MiniMax M2", "2.10", "8.40", "0.21", "0"),
            ("glm-4.7", "GLM-4.7", "2.00", "8.00", "0.40", "0"),
            ("glm-4.6", "GLM-4.6", "2.00", "8.00", "0.40", "0"),
            ("mimo-v2-flash", "Mimo V2 Flash", "0", "0", "0", "0"),
        ];

        for (model_id, display_name, input, output, cache_read, cache_creation) in pricing_data {
            conn.execute(
                "INSERT OR IGNORE INTO model_pricing (
                    model_id, display_name, input_cost_per_million, output_cost_per_million,
                    cache_read_cost_per_million, cache_creation_cost_per_million
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![model_id, display_name, input, output, cache_read, cache_creation],
            )
            .map_err(|e| AppError::Database(format!("插入模型定价失败: {e}")))?;
        }

        log::info!("已插入 {} 条默认模型定价数据", pricing_data.len());
        Ok(())
    }

    /// 确保模型定价表具备默认数据
    pub fn ensure_model_pricing_seeded(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        Self::seed_model_pricing(&conn)
    }

    // --- 辅助方法 ---

    pub(crate) fn get_user_version(conn: &Connection) -> Result<i32, AppError> {
        conn.query_row("PRAGMA user_version;", [], |row| row.get(0))
            .map_err(|e| AppError::Database(format!("读取 user_version 失败: {e}")))
    }

    pub(crate) fn set_user_version(conn: &Connection, version: i32) -> Result<(), AppError> {
        if version < 0 {
            return Err(AppError::Database("user_version 不能为负数".to_string()));
        }
        let sql = format!("PRAGMA user_version = {version};");
        conn.execute(&sql, [])
            .map_err(|e| AppError::Database(format!("写入 user_version 失败: {e}")))?;
        Ok(())
    }

    fn validate_identifier(s: &str, kind: &str) -> Result<(), AppError> {
        if s.is_empty() {
            return Err(AppError::Database(format!("{kind} 不能为空")));
        }
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(AppError::Database(format!(
                "非法{kind}: {s}，仅允许字母、数字和下划线"
            )));
        }
        Ok(())
    }

    pub(crate) fn table_exists(conn: &Connection, table: &str) -> Result<bool, AppError> {
        Self::validate_identifier(table, "表名")?;
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .map_err(|e| AppError::Database(format!("读取表名失败: {e}")))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(format!("查询表名失败: {e}")))?;
        while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let name: String = row
                .get(0)
                .map_err(|e| AppError::Database(format!("解析表名失败: {e}")))?;
            if name.eq_ignore_ascii_case(table) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn has_column(
        conn: &Connection,
        table: &str,
        column: &str,
    ) -> Result<bool, AppError> {
        Self::validate_identifier(table, "表名")?;
        Self::validate_identifier(column, "列名")?;
        let sql = format!("PRAGMA table_info(\"{table}\");");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(format!("读取表结构失败: {e}")))?;
        let mut rows = stmt
            .query([])
            .map_err(|e| AppError::Database(format!("查询表结构失败: {e}")))?;
        while let Some(row) = rows.next().map_err(|e| AppError::Database(e.to_string()))? {
            let name: String = row
                .get(1)
                .map_err(|e| AppError::Database(format!("读取列名失败: {e}")))?;
            if name.eq_ignore_ascii_case(column) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn add_column_if_missing(
        conn: &Connection,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<bool, AppError> {
        Self::validate_identifier(table, "表名")?;
        Self::validate_identifier(column, "列名")?;
        if !Self::table_exists(conn, table)? {
            return Ok(false);
        }
        if Self::has_column(conn, table, column)? {
            return Ok(false);
        }
        let sql = format!("ALTER TABLE \"{table}\" ADD COLUMN \"{column}\" {definition};");
        conn.execute(&sql, [])
            .map_err(|e| AppError::Database(format!("为表 {table} 添加列 {column} 失败: {e}")))?;
        log::info!("已为表 {table} 添加缺失列 {column}");
        Ok(true)
    }
}
