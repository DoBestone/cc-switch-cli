//! 流式健康检查服务
//!
//! 定期检测供应商的健康状态，支持重试机制。

use crate::app_config::AppType;
use crate::error::AppError;
use crate::provider::Provider;
use crate::store::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// 健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Failed,
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub success: bool,
    pub message: String,
    pub response_time_ms: Option<u64>,
    pub http_status: Option<u16>,
    pub model_used: String,
    pub tested_at: i64,
    pub retry_count: u32,
}

/// 流式检查配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamCheckConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub timeout_seconds: u64,
    pub retry_count: u32,
    pub retry_delay_seconds: u64,
    pub test_model: String,
}

impl Default for StreamCheckConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: 300, // 5 分钟
            timeout_seconds: 30,
            retry_count: 3,
            retry_delay_seconds: 5,
            test_model: "claude-3-haiku-20240307".to_string(),
        }
    }
}

/// 流式健康检查服务
pub struct StreamCheckService;

impl StreamCheckService {
    /// 检查单个供应商
    pub async fn check_provider(
        app_type: &AppType,
        provider: &Provider,
        config: &StreamCheckConfig,
    ) -> Result<HealthCheckResult, AppError> {
        let start = std::time::Instant::now();
        let mut last_error = String::new();
        let mut retry_count = 0;

        for attempt in 0..=config.retry_count {
            if attempt > 0 {
                tokio::time::sleep(tokio::time::Duration::from_secs(config.retry_delay_seconds)).await;
                retry_count += 1;
            }

            match Self::do_health_check(app_type, provider, config).await {
                Ok(result) => {
                    let response_time = start.elapsed().as_millis() as u64;
                    return Ok(HealthCheckResult {
                        status: if result.is_success { HealthStatus::Healthy } else { HealthStatus::Degraded },
                        success: true,
                        message: "健康检查通过".to_string(),
                        response_time_ms: Some(response_time),
                        http_status: Some(result.http_status),
                        model_used: config.test_model.clone(),
                        tested_at: Utc::now().timestamp(),
                        retry_count,
                    });
                }
                Err(e) => {
                    last_error = e.to_string();
                }
            }
        }

        Ok(HealthCheckResult {
            status: HealthStatus::Failed,
            success: false,
            message: last_error,
            response_time_ms: Some(start.elapsed().as_millis() as u64),
            http_status: None,
            model_used: config.test_model.clone(),
            tested_at: Utc::now().timestamp(),
            retry_count,
        })
    }

    async fn do_health_check(
        app_type: &AppType,
        provider: &Provider,
        config: &StreamCheckConfig,
    ) -> Result<HealthCheckResponse, AppError> {
        use crate::services::ProviderService;

        // 提取凭据
        let (api_key, base_url) = ProviderService::extract_credentials(provider, app_type)?;

        if api_key.is_empty() {
            return Err(AppError::Message("API Key 未配置".to_string()));
        }

        // 构建请求
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()?;

        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

        let response = client
            .post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": config.test_model,
                "max_tokens": 10,
                "messages": [{"role": "user", "content": "ping"}]
            }))
            .send()
            .await?;

        let http_status = response.status().as_u16();

        Ok(HealthCheckResponse {
            http_status,
            is_success: response.status().is_success(),
        })
    }

    /// 获取配置
    pub fn get_config(state: &AppState) -> Result<StreamCheckConfig, AppError> {
        let config_str = state.db.get_setting("stream_check_config")?;

        if let Some(s) = config_str {
            serde_json::from_str(&s)
                .map_err(|e| AppError::Config(format!("解析配置失败: {}", e)))
        } else {
            Ok(StreamCheckConfig::default())
        }
    }

    /// 保存配置
    pub fn save_config(state: &AppState, config: &StreamCheckConfig) -> Result<(), AppError> {
        let config_str = serde_json::to_string(config)
            .map_err(|e| AppError::Config(format!("序列化配置失败: {}", e)))?;

        state.db.set_setting("stream_check_config", &config_str)
    }
}

struct HealthCheckResponse {
    http_status: u16,
    is_success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = StreamCheckConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.interval_seconds, 300);
        assert_eq!(config.retry_count, 3);
    }
}