//! 端点测速服务模块
//!
//! 提供 API 端点的延迟测试功能。

use std::time::{Duration, Instant};

use crate::error::AppError;

/// 测速服务
pub struct SpeedtestService;

/// 测速结果
#[derive(Debug, Clone)]
pub struct SpeedtestResult {
    pub url: String,
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// 默认测试目标
const DEFAULT_TARGETS: &[&str] = &[
    "https://api.anthropic.com",
    "https://api.openai.com",
    "https://generativelanguage.googleapis.com",
];

impl SpeedtestService {
    /// 测试多个端点
    pub async fn test_endpoints(
        urls: Option<Vec<String>>,
        timeout_secs: u64,
        proxy: Option<&str>,
    ) -> Vec<SpeedtestResult> {
        let urls = urls.unwrap_or_else(|| {
            DEFAULT_TARGETS.iter().map(|s| s.to_string()).collect()
        });

        let timeout = Duration::from_secs(timeout_secs.clamp(2, 30));

        // 并发测试所有端点
        let futures: Vec<_> = urls
            .iter()
            .map(|url| Self::test_single(url, timeout, proxy))
            .collect();

        futures::future::join_all(futures).await
    }

    /// 测试单个端点
    async fn test_single(url: &str, timeout: Duration, proxy: Option<&str>) -> SpeedtestResult {
        let client_builder = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(5));

        let client_builder = if let Some(proxy_url) = proxy {
            match reqwest::Proxy::all(proxy_url) {
                Ok(p) => client_builder.proxy(p),
                Err(e) => {
                    return SpeedtestResult {
                        url: url.to_string(),
                        success: false,
                        latency_ms: None,
                        error: Some(format!("代理配置错误: {}", e)),
                    };
                }
            }
        } else {
            client_builder
        };

        let client = match client_builder.build() {
            Ok(c) => c,
            Err(e) => {
                return SpeedtestResult {
                    url: url.to_string(),
                    success: false,
                    latency_ms: None,
                    error: Some(format!("客户端构建失败: {}", e)),
                };
            }
        };

        // 热身请求（避免首包惩罚）
        let _ = client.head(url).send().await;

        // 正式测试
        let start = Instant::now();

        match client.head(url).send().await {
            Ok(resp) => {
                let latency = start.elapsed().as_millis() as u64;
                let success = resp.status().is_success()
                    || resp.status().as_u16() == 405
                    || resp.status().as_u16() == 401;

                SpeedtestResult {
                    url: url.to_string(),
                    success,
                    latency_ms: Some(latency),
                    error: if success {
                        None
                    } else {
                        Some(format!("HTTP {}", resp.status()))
                    },
                }
            }
            Err(e) => SpeedtestResult {
                url: url.to_string(),
                success: false,
                latency_ms: None,
                error: Some(e.to_string()),
            },
        }
    }

    /// 获取默认测试目标
    pub fn default_targets() -> Vec<String> {
        DEFAULT_TARGETS.iter().map(|s| s.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_speedtest_default_targets() {
        let targets = SpeedtestService::default_targets();
        assert!(!targets.is_empty());
    }
}
