//! 统一错误类型模块
//!
//! 定义应用中使用的所有错误类型，支持详细的错误上下文和链式错误追踪。

use std::path::Path;
use std::sync::PoisonError;
use thiserror::Error;

/// 应用统一错误类型
#[derive(Debug, Error)]
pub enum AppError {
    /// 配置相关错误
    #[error("配置错误: {0}")]
    Config(String),

    /// 无效输入
    #[error("无效输入: {0}")]
    InvalidInput(String),

    /// IO 错误（带路径上下文）
    #[error("IO 错误: {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// IO 错误（带自定义上下文）
    #[error("{context}: {source}")]
    IoContext {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// JSON 解析错误
    #[error("JSON 解析错误: {path}: {source}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },

    /// JSON 序列化错误
    #[error("JSON 序列化失败: {source}")]
    JsonSerialize {
        #[source]
        source: serde_json::Error,
    },

    /// TOML 解析错误
    #[error("TOML 解析错误: {path}: {source}")]
    Toml {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    /// 锁获取失败
    #[error("锁获取失败: {0}")]
    Lock(String),

    /// MCP 校验失败
    #[error("MCP 校验失败: {0}")]
    McpValidation(String),

    /// 通用消息错误
    #[error("{0}")]
    Message(String),

    /// 本地化错误（中英文双语）
    #[error("{zh} ({en})")]
    Localized {
        key: &'static str,
        zh: String,
        en: String,
    },

    /// 数据库错误
    #[error("数据库错误: {0}")]
    Database(String),

    /// 所有供应商熔断
    #[error("所有供应商已熔断，无可用渠道")]
    AllProvidersCircuitOpen,

    /// 未配置供应商
    #[error("未配置供应商")]
    NoProvidersConfigured,

    /// 供应商不存在
    #[error("供应商不存在: {0}")]
    ProviderNotFound(String),

    /// HTTP 请求错误
    #[error("HTTP 请求失败: {0}")]
    Http(String),
}

impl AppError {
    /// 创建 IO 错误
    pub fn io(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.as_ref().display().to_string(),
            source,
        }
    }

    /// 创建 JSON 解析错误
    pub fn json(path: impl AsRef<Path>, source: serde_json::Error) -> Self {
        Self::Json {
            path: path.as_ref().display().to_string(),
            source,
        }
    }

    /// 创建 TOML 解析错误
    pub fn toml(path: impl AsRef<Path>, source: toml::de::Error) -> Self {
        Self::Toml {
            path: path.as_ref().display().to_string(),
            source,
        }
    }

    /// 创建本地化错误
    pub fn localized(key: &'static str, zh: impl Into<String>, en: impl Into<String>) -> Self {
        Self::Localized {
            key,
            zh: zh.into(),
            en: en.into(),
        }
    }
}

impl<T> From<PoisonError<T>> for AppError {
    fn from(err: PoisonError<T>) -> Self {
        Self::Lock(err.to_string())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Database(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::IoContext {
            context: "IO 操作失败".to_string(),
            source: err,
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonSerialize { source: err }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(err.to_string())
    }
}

/// Result 类型别名
pub type Result<T> = std::result::Result<T, AppError>;
