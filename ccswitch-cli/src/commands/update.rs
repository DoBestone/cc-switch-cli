//! 更新检测与执行模块
//!
//! 自动检测新版本并提供一键更新功能。
//! 支持版本更新策略：大版本强制更新、中版本推荐更新、小版本选择性更新。

use anyhow::{bail, Result};
use colored::Colorize;
use semver::Version;
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;

use crate::output::OutputContext;

/// 版本更新类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType {
    /// 大版本更新（如 1.x.x -> 2.x.x）- 强制更新
    Major,
    /// 中版本更新（如 1.1.x -> 1.2.x）- 推荐更新
    Minor,
    /// 小版本更新（如 1.1.1 -> 1.1.2）- 选择性更新
    Patch,
    /// 无更新
    None,
}

impl UpdateType {
    /// 从版本差异判断更新类型
    pub fn from_versions(current: &Version, latest: &Version) -> Self {
        if latest.major > current.major {
            UpdateType::Major
        } else if latest.major == current.major && latest.minor > current.minor {
            UpdateType::Minor
        } else if latest.major == current.major
            && latest.minor == current.minor
            && latest.patch > current.patch
        {
            UpdateType::Patch
        } else {
            UpdateType::None
        }
    }

    /// 获取更新提示信息
    pub fn get_message(&self) -> &'static str {
        match self {
            UpdateType::Major => "🔴 大版本更新（建议立即更新）",
            UpdateType::Minor => "🟡 中版本更新（推荐更新）",
            UpdateType::Patch => "🟢 小版本更新（可选更新）",
            UpdateType::None => "已是最新版本",
        }
    }

    /// 是否需要强制更新
    pub fn is_forced(&self) -> bool {
        matches!(self, UpdateType::Major)
    }

    /// 是否需要提示用户
    #[allow(dead_code)]
    pub fn should_notify(&self) -> bool {
        !matches!(self, UpdateType::None)
    }
}

/// GitHub Release 信息
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[allow(dead_code)]
    published_at: String,
    body: Option<String>,
    assets: Vec<ReleaseAsset>,
    prerelease: bool,
    draft: bool,
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
    pub current: Version,
    pub latest: Version,
    pub has_update: bool,
    pub update_type: UpdateType,
    pub download_url: Option<String>,
    pub release_notes: Option<String>,
    pub release_url: String,
}

/// GitHub API 限流状态
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u32,
    pub remaining: u32,
    pub reset_time: Option<u64>,
}

const GITHUB_REPO: &str = "DoBestone/cc-switch-cli";
const GITHUB_API_URL: &str = "https://api.github.com/repos/DoBestone/cc-switch-cli/releases/latest";

/// 解析版本字符串，支持多种格式
fn parse_version(version_str: &str) -> Result<Version> {
    // 移除常见的版本前缀
    let cleaned = version_str
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .trim_start_matches("version-")
        .trim_start_matches("Version-");

    // 尝试直接解析
    if let Ok(v) = Version::parse(cleaned) {
        return Ok(v);
    }

    // 尝试提取版本号部分（处理可能的后缀如 -beta, -rc.1）
    let re = regex::Regex::new(r"(\d+\.\d+\.\d+)").unwrap();
    if let Some(caps) = re.captures(cleaned) {
        if let Ok(v) = Version::parse(&caps[1]) {
            return Ok(v);
        }
    }

    bail!("无法解析版本号: {}", version_str)
}

/// 检测新版本
#[allow(dead_code)]
pub async fn check_update(_ctx: &OutputContext) -> Result<Option<VersionInfo>> {
    check_update_internal().await
}

/// 内部版本检查实现
async fn check_update_internal() -> Result<Option<VersionInfo>> {
    log::debug!("正在检测更新...");

    let client = reqwest::Client::builder()
        .user_agent("cc-switch-cli")
        .timeout(Duration::from_secs(15))
        .build()?;

    let response = match client.get(GITHUB_API_URL).send().await {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = if e.is_timeout() {
                "网络请求超时，请检查网络连接".to_string()
            } else if e.is_connect() {
                "无法连接到 GitHub API，请检查网络".to_string()
            } else if e.is_request() {
                format!("请求失败: {}", e)
            } else {
                format!("网络错误: {}", e)
            };
            log::debug!("网络请求失败: {}", e);
            bail!("{}", error_msg);
        }
    };

    // 检查 GitHub API 限流
    let rate_limit = extract_rate_limit(&response);

    if !response.status().is_success() {
        let status = response.status();
        if status.as_u16() == 403 {
            if let Some(info) = rate_limit {
                if info.remaining == 0 {
                    bail!(
                        "GitHub API 请求次数已用尽，将在 {} 秒后重置",
                        info.reset_time.unwrap_or(0)
                    );
                }
            }
            bail!("GitHub API 访问被拒绝，请稍后重试");
        } else if status.as_u16() == 404 {
            bail!("未找到发布版本");
        }
        bail!("GitHub API 返回错误: {}", status);
    }

    // 记录限流信息
    if let Some(info) = &rate_limit {
        log::debug!(
            "GitHub API 限流状态: {}/{} 剩余",
            info.remaining,
            info.limit
        );
        if info.remaining < 5 {
            log::warn!(
                "GitHub API 请求次数即将用尽 ({}/{} 剩余)",
                info.remaining,
                info.limit
            );
        }
    }

    let release: GitHubRelease = response.json().await?;

    // 跳过预发布和草稿版本
    if release.prerelease || release.draft {
        log::debug!("跳过预发布或草稿版本");
        return Ok(None);
    }

    let current_version = parse_version(ccswitch_core::VERSION)?;
    let latest_version = parse_version(&release.tag_name)?;

    // 使用 semver 进行版本比较
    let has_update = latest_version > current_version;
    let update_type = UpdateType::from_versions(&current_version, &latest_version);

    // 获取适合当前平台的下载链接
    let download_url = get_platform_asset(&release.assets);

    let version_info = VersionInfo {
        current: current_version,
        latest: latest_version,
        has_update,
        update_type,
        download_url,
        release_notes: release.body,
        release_url: release.html_url,
    };

    Ok(Some(version_info))
}

/// 从响应头提取限流信息
fn extract_rate_limit(response: &reqwest::Response) -> Option<RateLimitInfo> {
    let headers = response.headers();

    let limit = headers
        .get("x-ratelimit-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())?;

    let remaining = headers
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())?;

    let reset_time = headers
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());

    Some(RateLimitInfo {
        limit,
        remaining,
        reset_time,
    })
}

/// 获取当前平台对应的下载资源
fn get_platform_asset(assets: &[ReleaseAsset]) -> Option<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let platform_suffix = match (os, arch) {
        ("macos", "x86_64") => "darwin-x86_64",
        ("macos", "aarch64") => "darwin-aarch64",
        ("linux", "x86_64") => "linux-x86_64-musl",  // 优先 musl 静态链接
        ("linux", "aarch64") => "linux-aarch64-musl",
        ("linux", "arm") => "linux-armv7",
        ("windows", "x86_64") => "windows-x86_64.exe",
        _ => return None,
    };

    // 优先查找 musl 版本，然后是标准版本
    let mut fallback_suffix: Option<&str> = None;
    if platform_suffix.contains("-musl") {
        fallback_suffix = Some(&platform_suffix[..platform_suffix.len() - 5]);
    }

    for asset in assets {
        if asset.name.contains(platform_suffix) {
            return Some(asset.browser_download_url.clone());
        }
    }

    // 回退到非 musl 版本
    if let Some(fallback) = fallback_suffix {
        for asset in assets {
            if asset.name.contains(fallback) && !asset.name.contains("-musl") {
                return Some(asset.browser_download_url.clone());
            }
        }
    }

    None
}

/// 显示版本状态
pub async fn show_status(_ctx: &OutputContext, check_only: bool) -> Result<()> {
    match check_update_internal().await {
        Ok(Some(info)) => {
            println!();
            if info.has_update {
                print_update_notification(&info, check_only);
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

/// 打印更新通知
fn print_update_notification(info: &VersionInfo, check_only: bool) {
    println!("{}", "╔════════════════════════════════════════╗".green());
    println!(
        "{}",
        format!("║           🎉 发现新版本!               ║").green().bold()
    );
    println!("{}", "╚════════════════════════════════════════╝".green());
    println!();

    // 显示版本更新类型
    println!("  {}", info.update_type.get_message().bold());
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
        print_update_instructions(&info.update_type);
    }
}

/// 打印更新说明
fn print_update_instructions(update_type: &UpdateType) {
    match update_type {
        UpdateType::Major => {
            println!("{}", "⚠️  大版本更新可能包含不兼容变更，请查看更新说明".yellow());
            println!();
            println!("{}", "运行以下命令更新:".white().bold());
            println!("  {}", "cc-switch self-update".green());
        }
        UpdateType::Minor => {
            println!("{}", "运行以下命令更新:".white());
            println!("  {}", "cc-switch self-update".green());
        }
        UpdateType::Patch => {
            println!("{}", "运行以下命令更新:".white());
            println!("  {}", "cc-switch self-update".green());
        }
        UpdateType::None => {}
    }
    println!();
    println!("{}", "或重新运行安装脚本:".white());
    println!(
        "  {}",
        "curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/install.sh | bash"
            .dimmed()
    );
}

/// 启动时检查版本（静默模式）
pub async fn check_on_startup() {
    // 仅在后台静默检查，失败不报错
    match check_update_internal().await {
        Ok(Some(info)) if info.has_update => {
            println!();
            println!(
                "{}",
                format!("╭─ {} ─────────────────────────╮", info.update_type.get_message())
                    .cyan()
            );
            println!(
                "{}",
                format!("│ 当前: v{} → 最新: v{} │", info.current, info.latest)
                    .cyan()
            );
            println!(
                "{}",
                "│ 运行 cc-switch self-update 更新  │".cyan()
            );
            println!(
                "{}",
                "╰─────────────────────────────────╯".cyan()
            );
            println!();
        }
        _ => {}
    }
}

/// 执行自动更新
pub async fn self_update(_ctx: &OutputContext, force: bool) -> Result<()> {
    // 首先检查是否有新版本
    let version_info = match check_update_internal().await? {
        Some(info) => info,
        None => {
            println!("{}", "无法获取版本信息".yellow());
            return Ok(());
        }
    };

    // 大版本更新强制提示
    if version_info.update_type.is_forced() && !force {
        println!();
        println!("{}", "⚠️  检测到大版本更新！".red().bold());
        println!(
            "{}",
            format!(
                "  {} → {}",
                format!("v{}", version_info.current).yellow(),
                format!("v{}", version_info.latest).green().bold()
            )
        );
        println!();
        println!(
            "{}",
            "大版本更新可能包含不兼容变更。建议先查看更新说明：".yellow()
        );
        println!("  {}", version_info.release_url.blue().underline());
        println!();
        println!("{}", "确认更新？(y/N)".white());

        // 简单的确认提示（非交互模式下直接继续）
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        if let Some(Ok(line)) = stdin.lock().lines().next() {
            if line.to_lowercase() != "y" && line.to_lowercase() != "yes" {
                println!("{}", "已取消更新".yellow());
                return Ok(());
            }
        }
    }

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
                println!(
                    "{}",
                    format!("下载失败: {}，尝试从源码编译...", e).yellow()
                );
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
                "curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/install.sh | bash"
                    .cyan()
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
        .timeout(Duration::from_secs(120))
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

    // 验证新二进制
    #[cfg(unix)]
    {
        let output = Command::new(&temp_path)
            .arg("--version")
            .output();
        if let Ok(output) = output {
            if !output.status.success() {
                std::fs::remove_file(&temp_path)?;
                bail!("下载的二进制文件无法执行");
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert!(parse_version("1.2.3").is_ok());
        assert!(parse_version("v1.2.3").is_ok());
        assert!(parse_version("V1.2.3").is_ok());
        assert!(parse_version("1.2.3-beta").is_ok());
        assert!(parse_version("1.2.3-rc.1").is_ok());
    }

    #[test]
    fn test_update_type() {
        let v1_0_0 = Version::parse("1.0.0").unwrap();
        let v1_0_1 = Version::parse("1.0.1").unwrap();
        let v1_1_0 = Version::parse("1.1.0").unwrap();
        let v2_0_0 = Version::parse("2.0.0").unwrap();

        assert_eq!(UpdateType::from_versions(&v1_0_0, &v1_0_1), UpdateType::Patch);
        assert_eq!(UpdateType::from_versions(&v1_0_0, &v1_1_0), UpdateType::Minor);
        assert_eq!(UpdateType::from_versions(&v1_0_0, &v2_0_0), UpdateType::Major);
        assert_eq!(UpdateType::from_versions(&v1_0_0, &v1_0_0), UpdateType::None);
    }

    #[test]
    fn test_update_type_message() {
        assert!(UpdateType::Major.get_message().contains("大版本"));
        assert!(UpdateType::Minor.get_message().contains("中版本"));
        assert!(UpdateType::Patch.get_message().contains("小版本"));
    }
}