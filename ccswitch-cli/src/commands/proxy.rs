//! 代理命令模块
//!
//! 实现全局代理设置的 CLI 命令。

use anyhow::Result;
use ccswitch_core::{AppState, ProxyService};

use crate::output::{print_error, print_info, print_success, OutputContext};

/// 获取当前代理设置
pub fn get(_ctx: &OutputContext) -> Result<()> {
    let state = AppState::init()?;

    match ProxyService::get(&state)? {
        Some(proxy) => {
            println!("\n🌐 当前代理设置\n");
            println!("代理地址: {}", proxy);
        }
        None => {
            print_info("未设置全局代理");
        }
    }

    Ok(())
}

/// 设置全局代理
pub fn set(_ctx: &OutputContext, url: &str) -> Result<()> {
    let state = AppState::init()?;

    ProxyService::set(&state, url)?;

    print_success(&format!("已设置全局代理: {}", url));

    Ok(())
}

/// 清除全局代理设置
pub fn clear(_ctx: &OutputContext) -> Result<()> {
    let state = AppState::init()?;

    ProxyService::clear(&state)?;

    print_success("已清除全局代理设置");

    Ok(())
}

/// 测试代理连接
pub async fn test(_ctx: &OutputContext, proxy_url: Option<String>) -> Result<()> {
    let state = AppState::init()?;

    // 使用提供的代理或当前设置的代理
    let proxy = match proxy_url {
        Some(url) => Some(url),
        None => ProxyService::get(&state)?,
    };

    println!("\n🔍 测试代理连接...\n");

    let results = ProxyService::test(proxy.as_deref(), None).await;

    println!("{:<45} {:<10} {:<10}", "目标", "状态", "延迟");
    println!("{}", "-".repeat(65));

    for result in &results {
        let status = if result.success { "✓ 成功" } else { "✗ 失败" };
        let latency = result
            .latency_ms
            .map(|ms| format!("{}ms", ms))
            .unwrap_or_else(|| "-".to_string());

        println!("{:<45} {:<10} {:<10}", result.url, status, latency);

        if let Some(err) = &result.error {
            println!("  错误: {}", err);
        }
    }

    Ok(())
}

/// 扫描本地代理
pub async fn scan(_ctx: &OutputContext) -> Result<()> {
    println!("\n🔍 扫描本地代理...\n");

    let found = ProxyService::scan_local().await;

    if found.is_empty() {
        print_info("未发现本地代理");
    } else {
        println!("发现 {} 个本地代理:\n", found.len());
        for proxy in &found {
            println!("  {}", proxy);
        }
        println!();
        print_info("使用 'cc-switch proxy set <url>' 设置代理");
    }

    Ok(())
}
