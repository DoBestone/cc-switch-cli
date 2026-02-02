//! 应用状态模块
//!
//! 封装全局应用状态，包括数据库连接等共享资源。

use crate::database::Database;
use crate::error::AppError;
use std::sync::Arc;

/// 全局应用状态
pub struct AppState {
    /// 数据库连接
    pub db: Arc<Database>,
}

impl AppState {
    /// 初始化应用状态
    pub fn init() -> Result<Self, AppError> {
        let db = Database::init()?;
        Ok(Self { db: Arc::new(db) })
    }

    /// 使用内存数据库创建（用于测试）
    pub fn memory() -> Result<Self, AppError> {
        let db = Database::memory()?;
        Ok(Self { db: Arc::new(db) })
    }

    /// 使用自定义数据库创建
    pub fn with_database(db: Database) -> Self {
        Self { db: Arc::new(db) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_memory() {
        let state = AppState::memory().unwrap();
        assert!(state
            .db
            .get_all_providers("claude")
            .unwrap()
            .is_empty());
    }
}
