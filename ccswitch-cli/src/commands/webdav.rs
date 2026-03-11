//! WebDAV 同步命令
//!
//! 管理配置的云端同步。

use anyhow::{anyhow, Result};
use ccswitch_core::{AppState, WebDavSyncService};
use crate::output::{print_success, print_info, print_warning};
use crate::cli::OutputFormat;
use crate::output::OutputContext;

/// 显示 WebDAV 配置
pub fn show_config(ctx: &OutputContext) -> Result<()> {
    let state = AppState::init()?;
    let settings = WebDavSyncService::get_settings(&state)?;

    if let Some(s) = settings {
        match ctx.format {
            OutputFormat::Json => {
                // 隐藏密码
                let mut s_redacted = s.clone();
                if !s_redacted.password.is_empty() {
                    s_redacted.password = "***".to_string();
                }
                let json = serde_json::to_string_pretty(&s_redacted)?;
                println!("{}", json);
            }
            _ => {
                print_info("WebDAV 同步配置:");
                println!("  状态: {}", if s.enabled { "已启用" } else { "已禁用" });
                println!("  URL: {}", s.base_url);
                println!("  用户名: {}", s.username);
                println!("  远程目录: {}", s.remote_root);
                println!("  配置文件: {}", s.profile);
                if s.auto_sync_enabled {
                    println!("  自动同步: 每 {} 分钟", s.sync_interval_minutes);
                }
                if let Some(t) = s.last_sync_at {
                    let dt = chrono::DateTime::from_timestamp(t, 0)
                        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| t.to_string());
                    println!("  最后同步: {}", dt);
                }
            }
        }
    } else {
        print_warning("WebDAV 同步未配置");
    }

    Ok(())
}

/// 配置 WebDAV
pub fn configure(
    url: &str,
    username: &str,
    password: &str,
    remote_root: Option<&str>,
) -> Result<()> {
    let state = AppState::init()?;

    let mut settings = WebDavSyncService::get_settings(&state)?
        .unwrap_or_default();

    settings.base_url = url.to_string();
    settings.username = username.to_string();
    settings.password = password.to_string();
    if let Some(r) = remote_root {
        settings.remote_root = r.to_string();
    }
    settings.normalize();

    WebDavSyncService::save_settings(&state, &settings)?;
    print_success("WebDAV 配置已保存");

    Ok(())
}

/// 启用/禁用同步
pub fn toggle(enable: bool) -> Result<()> {
    let state = AppState::init()?;

    let mut settings = WebDavSyncService::get_settings(&state)?
        .ok_or_else(|| anyhow!("请先配置 WebDAV"))?;

    settings.enabled = enable;
    WebDavSyncService::save_settings(&state, &settings)?;

    if enable {
        print_success("WebDAV 同步已启用");
    } else {
        print_success("WebDAV 同步已禁用");
    }

    Ok(())
}

/// 测试连接
pub fn test() -> Result<()> {
    let state = AppState::init()?;

    let settings = WebDavSyncService::get_settings(&state)?
        .ok_or_else(|| anyhow!("请先配置 WebDAV"))?;

    print_info("正在测试 WebDAV 连接...");

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        WebDavSyncService::test_connection(&settings).await
    })?;

    print_success("WebDAV 连接测试成功");
    Ok(())
}

/// 上传配置
pub fn upload(_ctx: &OutputContext) -> Result<()> {
    let state = AppState::init()?;

    print_info("正在上传配置到 WebDAV...");

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        WebDavSyncService::upload(&state).await
    })?;

    print_success("配置已上传到 WebDAV");
    Ok(())
}

/// 下载配置
pub fn download(ctx: &OutputContext) -> Result<()> {
    let state = AppState::init()?;

    print_info("正在从 WebDAV 下载配置...");

    let rt = tokio::runtime::Runtime::new()?;
    let config = rt.block_on(async {
        WebDavSyncService::download(&state).await
    })?;

    match ctx.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&config)?;
            println!("{}", json);
        }
        _ => {
            print_success("配置下载成功");
            // TODO: 应用配置
        }
    }

    Ok(())
}

/// 显示远程信息
pub fn remote_info(ctx: &OutputContext) -> Result<()> {
    let state = AppState::init()?;

    let settings = WebDavSyncService::get_settings(&state)?
        .ok_or_else(|| anyhow!("请先配置 WebDAV"))?;

    if !settings.enabled {
        return Err(anyhow!("WebDAV 同步未启用"));
    }

    let rt = tokio::runtime::Runtime::new()?;
    let info = rt.block_on(async {
        WebDavSyncService::fetch_remote_info(&settings).await
    })?;

    if let Some(i) = info {
        match ctx.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&i)?;
                println!("{}", json);
            }
            _ => {
                if i.get("exists").and_then(|v| v.as_bool()).unwrap_or(false) {
                    print_info("远程配置存在");
                    if let Some(lm) = i.get("lastModified").and_then(|v| v.as_str()) {
                        println!("  最后修改: {}", lm);
                    }
                } else {
                    print_warning("远程配置不存在");
                }
            }
        }
    }

    Ok(())
}