//! 更新检测与执行模块
//!
//! 自动检测新版本并提供一键更新功能。

use anyhow::{bail, Result};
use colored::Colorize;
use serde::Deserialize;
use std::process::Command;

use crate::output::OutputContext;

/// GitHub Release 信息
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[allow(dead_code)]
    published_at: String,
    body: Option<String>,
    assets: Vec<ReleaseAsset>,
}

/// Release 资源
#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    #[allow(dead_code)]
    size: u64,
}

/// 版本比较结果
#[derive(Debug)]
pub struct VersionInfo {
    pub current: String,
    pub latest: String,
    pub has_update: bool,
    pub download_url: Option<String>,
    pub release_notes: Option<String>,
    pub release_url: String,
}

const GITHUB_REPO: &str = "DoBestone/cc-switch-cli";
const GITHUB_API_URL: &str = "https://api.github.com/repos/DoBestone/cc-switch-cli/releases/latest";

/// 检测新版本
pub async fn check_update(_ctx: &OutputContext) -> Result<Option<VersionInfo>> {
    println!("{}", "正在检测更新...".dimmed());

    let client = reqwest::Client::builder()
        .user_agent("cc-switch-cli")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = match client.get(GITHUB_API_URL).send().await {
        Ok(resp) => resp,
        Err(e) => {
            log::debug!("网络请求失败: {}", e);
            bail!("无法连接到 GitHub API: {}", e);
        }
    };

    if !response.status().is_success() {
        bail!("GitHub API 返回错误: {}", response.status());
    }

    let release: GitHubRelease = response.json().await?;
    let current_version = ccswitch_core::VERSION;
    let latest_version = release.tag_name.trim_start_matches('v');

    // 比较版本
    let has_update = compare_versions(current_version, latest_version);

    // 获取适合当前平台的下载链接
    let download_url = get_platform_asset(&release.assets);

    let version_info = VersionInfo {
        current: current_version.to_string(),
        latest: latest_version.to_string(),
        has_update,
        download_url,
        release_notes: release.body,
        release_url: release.html_url,
    };

    Ok(Some(version_info))
}

/// 比较版本号（语义化版本）
fn compare_versions(current: &str, latest: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };

    let current_parts = parse_version(current);
    let latest_parts = parse_version(latest);

    for i in 0..3 {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let l = latest_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

/// 获取当前平台对应的下载资源
fn get_platform_asset(assets: &[ReleaseAsset]) -> Option<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let platform_suffix = match (os, arch) {
        ("macos", "x86_64") => "darwin-x86_64",
        ("macos", "aarch64") => "darwin-aarch64",
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("linux", "arm") => "linux-armv7",
        ("windows", "x86_64") => "windows-x86_64.exe",
        _ => return None,
    };

    assets
        .iter()
        .find(|a| a.name.contains(platform_suffix))
        .map(|a| a.browser_download_url.clone())
}

/// 显示版本状态
pub async fn show_status(ctx: &OutputContext, check_only: bool) -> Result<()> {
    match check_update(ctx).await {
        Ok(Some(info)) => {
            println!();
            if info.has_update {
                println!("{}", "╔════════════════════════════════════════╗".green());
                println!("{}", "║           🎉 发现新版本!               ║".green().bold());
                println!("{}", "╚════════════════════════════════════════╝".green());
                println!();
                println!(
                    "  当前版本: {}",
                    format!("v{}", info.current).yellow()
                );
                println!(
                    "  最新版本: {}",
                    format!("v{}", info.latest).green().bold()
                );
                println!();

                if let Some(notes) = &info.release_notes {
                    let short_notes: String = notes.lines().take(5).collect::<Vec<_>>().join("\n");
                    if !short_notes.is_empty() {
                        println!("{}", "更新说明:".cyan());
                        for line in short_notes.lines() {
                            println!("  {}", line.dimmed());
                        }
                        if notes.lines().count() > 5 {
                            println!("  {}", "...".dimmed());
                        }
                        println!();
                    }
                }

                println!("  详情: {}", info.release_url.blue().underline());
                println!();

                if !check_only {
                    println!("{}", "运行以下命令更新:".white());
                    println!("  {}", "cc-switch self-update".green());
                    println!();
                    println!("{}", "或重新运行安装脚本:".white());
                    println!("  {}", "curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/install.sh | bash".dimmed());
                }
            } else {
                println!("{}", "✓ 已是最新版本".green());
                println!(
                    "  当前版本: {}",
                    format!("v{}", info.current).green()
                );
            }
            println!();
            Ok(())
        }
        Ok(None) => {
            println!("{}", "无法获取版本信息".yellow());
            Ok(())
        }
        Err(e) => {
            println!("{}", format!("检测更新失败: {}", e).red());
            Err(e)
        }
    }
}

/// 执行自动更新
pub async fn self_update(ctx: &OutputContext, force: bool) -> Result<()> {
    // 首先检查是否有新版本
    let version_info = match check_update(ctx).await? {
        Some(info) => info,
        None => {
            println!("{}", "无法获取版本信息".yellow());
            return Ok(());
        }
    };

    if !version_info.has_update && !force {
        println!("{}", "✓ 已是最新版本，无需更新".green());
        println!(
            "  当前版本: {}",
            format!("v{}", version_info.current).green()
        );
        return Ok(());
    }

    println!();
    println!("{}", "╔════════════════════════════════════════╗".cyan());
    println!("{}", "║           🔄 开始更新...               ║".cyan().bold());
    println!("{}", "╚════════════════════════════════════════╝".cyan());
    println!();
    println!(
        "  {} → {}",
        format!("v{}", version_info.current).yellow(),
        format!("v{}", version_info.latest).green()
    );
    println!();

    // 尝试使用预编译二进制更新
    if let Some(download_url) = &version_info.download_url {
        println!("{}", "正在下载预编译二进制...".dimmed());

        match download_and_install(download_url).await {
            Ok(()) => {
                println!();
                println!("{}", "╔════════════════════════════════════════╗".green());
                println!("{}", "║           ✓ 更新成功!                  ║".green().bold());
                println!("{}", "╚════════════════════════════════════════╝".green());
                println!();
                println!(
                    "  新版本: {}",
                    format!("v{}", version_info.latest).green()
                );
                println!();
                return Ok(());
            }
            Err(e) => {
                println!("{}", format!("下载失败: {}，尝试从源码编译...", e).yellow());
            }
        }
    }

    // 回退：使用 cargo install 更新
    println!("{}", "使用 cargo 从源码编译更新...".dimmed());
    println!("{}", "(这可能需要几分钟)".dimmed());
    println!();

    let status = Command::new("cargo")
        .args([
            "install",
            "--git",
            &format!("https://github.com/{}.git", GITHUB_REPO),
            "--force",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!();
            println!("{}", "╔════════════════════════════════════════╗".green());
            println!("{}", "║           ✓ 更新成功!                  ║".green().bold());
            println!("{}", "╚════════════════════════════════════════╝".green());
            println!();
            Ok(())
        }
        Ok(s) => {
            bail!("cargo install 失败，退出码: {:?}", s.code())
        }
        Err(e) => {
            println!("{}", format!("运行 cargo 失败: {}", e).red());
            println!();
            println!("{}", "请手动运行以下命令更新:".white());
            println!(
                "  {}",
                "curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/install.sh | bash".cyan()
            );
            println!();
            bail!("自动更新失败: {}", e)
        }
    }
}

/// 下载并安装预编译二进制
async fn download_and_install(url: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("cc-switch-cli")
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    // 下载
    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        bail!("下载失败: HTTP {}", response.status());
    }

    let bytes = response.bytes().await?;

    // 获取当前可执行文件路径
    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe.parent().ok_or_else(|| {
        anyhow::anyhow!("无法获取可执行文件目录")
    })?;

    // 创建临时文件
    let temp_path = exe_dir.join(".cc-switch-update");
    std::fs::write(&temp_path, &bytes)?;

    // 设置可执行权限 (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o755))?;
    }

    // 备份当前版本
    let backup_path = exe_dir.join(".cc-switch-backup");
    if backup_path.exists() {
        std::fs::remove_file(&backup_path)?;
    }
    std::fs::rename(&current_exe, &backup_path)?;

    // 替换新版本
    match std::fs::rename(&temp_path, &current_exe) {
        Ok(()) => {
            // 删除备份
            let _ = std::fs::remove_file(&backup_path);
            Ok(())
        }
        Err(e) => {
            // 恢复备份
            let _ = std::fs::rename(&backup_path, &current_exe);
            bail!("替换可执行文件失败: {}", e)
        }
    }
}

/// 显示版本和更新信息
#[allow(dead_code)]
pub fn show_version() {
    println!("cc-switch {}", ccswitch_core::VERSION);
    println!();
    println!("{}", "检查更新请运行:".dimmed());
    println!("  {}", "cc-switch self-update --check".cyan());
}
