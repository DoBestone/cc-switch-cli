//! 全局代理服务模块
//!
//! 提供代理设置的管理功能，包括设置、测试和扫描本地代理。

use std::time::Duration;

use crate::error::AppError;
use crate::store::AppState;

/// 代理服务
pub struct ProxyService;

/// 代理测试结果
#[derive(Debug, Clone)]
pub struct ProxyTestResult {
    pub url: String,
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// 常见的测试目标
const TEST_TARGETS: &[&str] = &[
    "https://api.anthropic.com",
    "https://api.openai.com",
    "https://generativelanguage.googleapis.com",
];

/// 常见的本地代理端口
const COMMON_PROXY_PORTS: &[u16] = &[
    1080, 1081, 1087, 7890, 7891, 7892, 7893, 8080, 8118, 8888, 9050, 9150,
];

impl ProxyService {
    /// 获取当前代理设置
    pub fn get(state: &AppState) -> Result<Option<String>, AppError> {
        state.db.get_setting("global_proxy")
    }

    /// 设置全局代理
    pub fn set(state: &AppState, proxy_url: &str) -> Result<(), AppError> {
        // 验证代理 URL 格式
        Self::validate_proxy_url(proxy_url)?;

        state.db.set_setting("global_proxy", proxy_url)?;

        Ok(())
    }

    /// 清除全局代理设置
    pub fn clear(state: &AppState) -> Result<(), AppError> {
        state.db.delete_setting("global_proxy")?;
        Ok(())
    }

    /// 验证代理 URL 格式
    fn validate_proxy_url(url: &str) -> Result<(), AppError> {
        let url_lower = url.to_lowercase();

        // 检查协议
        if !url_lower.starts_with("http://")
            && !url_lower.starts_with("https://")
            && !url_lower.starts_with("socks5://")
            && !url_lower.starts_with("socks5h://")
        {
            return Err(AppError::InvalidInput(
                "代理 URL 必须以 http://, https://, socks5:// 或 socks5h:// 开头".to_string(),
            ));
        }

        // 尝试解析 URL
        url::Url::parse(url).map_err(|e| AppError::InvalidInput(format!("无效的代理 URL: {}", e)))?;

        Ok(())
    }

    /// 测试代理连接
    pub async fn test(proxy_url: Option<&str>, targets: Option<Vec<String>>) -> Vec<ProxyTestResult> {
        let targets = targets.unwrap_or_else(|| {
            TEST_TARGETS.iter().map(|s| s.to_string()).collect()
        });

        let mut results = Vec::new();

        for target in targets {
            let result = Self::test_single(proxy_url, &target).await;
            results.push(result);
        }

        results
    }

    /// 测试单个目标
    async fn test_single(proxy_url: Option<&str>, target: &str) -> ProxyTestResult {
        let client_builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5));

        let client = if let Some(proxy) = proxy_url {
            match reqwest::Proxy::all(proxy) {
                Ok(p) => client_builder.proxy(p),
                Err(e) => {
                    return ProxyTestResult {
                        url: target.to_string(),
                        success: false,
                        latency_ms: None,
                        error: Some(format!("代理配置错误: {}", e)),
                    };
                }
            }
        } else {
            client_builder
        };

        let client = match client.build() {
            Ok(c) => c,
            Err(e) => {
                return ProxyTestResult {
                    url: target.to_string(),
                    success: false,
                    latency_ms: None,
                    error: Some(format!("客户端构建失败: {}", e)),
                };
            }
        };

        let start = std::time::Instant::now();

        match client.head(target).send().await {
            Ok(resp) => {
                let latency = start.elapsed().as_millis() as u64;
                ProxyTestResult {
                    url: target.to_string(),
                    success: resp.status().is_success() || resp.status().as_u16() == 405,
                    latency_ms: Some(latency),
                    error: None,
                }
            }
            Err(e) => ProxyTestResult {
                url: target.to_string(),
                success: false,
                latency_ms: None,
                error: Some(e.to_string()),
            },
        }
    }

    /// 扫描本地代理
    pub async fn scan_local() -> Vec<String> {
        let mut found = Vec::new();

        for port in COMMON_PROXY_PORTS {
            // 测试 HTTP 代理
            let http_proxy = format!("http://127.0.0.1:{}", port);
            if Self::check_proxy_available(&http_proxy).await {
                found.push(http_proxy);
                continue;
            }

            // 测试 SOCKS5 代理
            let socks_proxy = format!("socks5://127.0.0.1:{}", port);
            if Self::check_proxy_available(&socks_proxy).await {
                found.push(socks_proxy);
            }
        }

        found
    }

    /// 检查代理是否可用
    async fn check_proxy_available(proxy_url: &str) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .connect_timeout(Duration::from_secs(1))
            .proxy(reqwest::Proxy::all(proxy_url).unwrap())
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        // 尝试连接一个简单的目标
        client
            .head("https://www.google.com")
            .send()
            .await
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_proxy_url() {
        assert!(ProxyService::validate_proxy_url("http://127.0.0.1:7890").is_ok());
        assert!(ProxyService::validate_proxy_url("https://proxy.example.com:8080").is_ok());
        assert!(ProxyService::validate_proxy_url("socks5://127.0.0.1:1080").is_ok());
        assert!(ProxyService::validate_proxy_url("socks5h://127.0.0.1:1080").is_ok());

        assert!(ProxyService::validate_proxy_url("ftp://127.0.0.1:21").is_err());
        assert!(ProxyService::validate_proxy_url("invalid").is_err());
    }
}