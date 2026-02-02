//! 配置文件路径和读写模块
//!
//! 处理各类配置文件的路径解析和原子读写操作。

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 获取用户主目录
///
/// 支持 HOME 环境变量覆盖（用于测试隔离）
pub fn get_home_dir() -> PathBuf {
    // 支持测试环境下的路径覆盖
    if let Ok(home) = std::env::var("CCSWITCH_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    #[cfg(windows)]
    if let Ok(home) = std::env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    dirs::home_dir().unwrap_or_else(|| {
        log::warn!("无法获取用户主目录，回退到当前目录");
        PathBuf::from(".")
    })
}

/// 获取 Claude Code 配置目录路径
///
/// 默认: `~/.claude`
pub fn get_claude_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CCSWITCH_CLAUDE_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    get_home_dir().join(".claude")
}

/// 获取 Claude MCP 配置文件路径
///
/// 默认: `~/.claude.json`
pub fn get_claude_mcp_path() -> PathBuf {
    if let Ok(path) = std::env::var("CCSWITCH_CLAUDE_MCP_PATH") {
        return PathBuf::from(path);
    }
    get_home_dir().join(".claude.json")
}

/// 获取 Claude Code 主配置文件路径
///
/// 优先使用 `settings.json`，兼容旧版 `claude.json`
pub fn get_claude_settings_path() -> PathBuf {
    let dir = get_claude_config_dir();
    let settings = dir.join("settings.json");
    if settings.exists() {
        return settings;
    }
    // 兼容旧版命名
    let legacy = dir.join("claude.json");
    if legacy.exists() {
        return legacy;
    }
    // 默认使用新命名
    settings
}

/// 获取 Codex 配置目录路径
///
/// 默认: `~/.codex`
pub fn get_codex_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CCSWITCH_CODEX_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    get_home_dir().join(".codex")
}

/// 获取 Codex 配置文件路径
pub fn get_codex_config_path() -> PathBuf {
    get_codex_config_dir().join("config.toml")
}

/// 获取 Codex auth.toml 路径
pub fn get_codex_auth_path() -> PathBuf {
    get_codex_config_dir().join("auth.toml")
}

/// 获取 Gemini CLI 配置目录路径
///
/// 默认: `~/.gemini`
pub fn get_gemini_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CCSWITCH_GEMINI_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    get_home_dir().join(".gemini")
}

/// 获取 Gemini 配置文件路径
pub fn get_gemini_settings_path() -> PathBuf {
    get_gemini_config_dir().join("settings.json")
}

/// 获取 OpenCode 配置目录路径
///
/// 默认: `~/.opencode`
pub fn get_opencode_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CCSWITCH_OPENCODE_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    get_home_dir().join(".opencode")
}

/// 获取应用配置目录路径
///
/// 默认: `~/.cc-switch`
/// Linux 服务器建议: `~/.config/cc-switch` 或使用默认
pub fn get_app_config_dir() -> PathBuf {
    // 支持环境变量覆盖
    if let Ok(dir) = std::env::var("CCSWITCH_CONFIG_DIR") {
        return PathBuf::from(dir);
    }

    // XDG Base Directory 规范支持 (Linux)
    #[cfg(target_os = "linux")]
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg_config).join("cc-switch");
    }

    // 默认使用 ~/.cc-switch（与原项目保持兼容）
    get_home_dir().join(".cc-switch")
}

/// 获取应用配置文件路径
pub fn get_app_config_path() -> PathBuf {
    get_app_config_dir().join("config.json")
}

/// 获取数据库文件路径
pub fn get_database_path() -> PathBuf {
    get_app_config_dir().join("cc-switch.db")
}

/// 读取 JSON 配置文件
pub fn read_json_file<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, AppError> {
    if !path.exists() {
        return Err(AppError::Config(format!("文件不存在: {}", path.display())));
    }

    let content = fs::read_to_string(path).map_err(|e| AppError::io(path, e))?;

    serde_json::from_str(&content).map_err(|e| AppError::json(path, e))
}

/// 写入 JSON 配置文件（原子写入）
pub fn write_json_file<T: Serialize>(path: &Path, data: &T) -> Result<(), AppError> {
    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let json =
        serde_json::to_string_pretty(data).map_err(|e| AppError::JsonSerialize { source: e })?;

    atomic_write(path, json.as_bytes())
}

/// 写入文本文件（原子写入）
pub fn write_text_file(path: &Path, data: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    atomic_write(path, data.as_bytes())
}

/// 原子写入：写入临时文件后 rename 替换，避免半写状态
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    let parent = path
        .parent()
        .ok_or_else(|| AppError::Config("无效的路径".to_string()))?;

    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::Config("无效的文件名".to_string()))?
        .to_string_lossy()
        .to_string();

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let mut tmp = parent.to_path_buf();
    tmp.push(format!("{file_name}.tmp.{ts}"));

    {
        let mut f = fs::File::create(&tmp).map_err(|e| AppError::io(&tmp, e))?;
        f.write_all(data).map_err(|e| AppError::io(&tmp, e))?;
        f.flush().map_err(|e| AppError::io(&tmp, e))?;
    }

    // Unix: 保留原文件权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let perm = meta.permissions().mode();
            let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(perm));
        }
    }

    // 原子替换
    fs::rename(&tmp, path).map_err(|e| AppError::IoContext {
        context: format!("原子替换失败: {} -> {}", tmp.display(), path.display()),
        source: e,
    })?;

    Ok(())
}

/// 清理供应商名称，确保文件名安全
pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        let data = r#"{"key": "value"}"#;
        atomic_write(&path, data.as_bytes()).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, data);
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("Test/Name"), "test-name");
        assert_eq!(sanitize_name("my:provider"), "my-provider");
        assert_eq!(sanitize_name("normal-name"), "normal-name");
    }
}
