//! 输出格式化模块
//!
//! 处理表格、JSON、YAML 等输出格式。

use colored::Colorize;
use serde::Serialize;
use tabled::{settings::Style, Table, Tabled};

use crate::cli::OutputFormat;

/// 输出上下文
pub struct OutputContext {
    pub format: OutputFormat,
    #[allow(dead_code)]
    pub no_color: bool,
}

impl OutputContext {
    pub fn new(format: OutputFormat, no_color: bool) -> Self {
        // 如果禁用颜色，设置环境变量
        if no_color {
            colored::control::set_override(false);
        }
        Self { format, no_color }
    }
}

/// 供应商列表行
#[derive(Tabled, Serialize)]
pub struct ProviderRow {
    #[tabled(rename = "ID")]
    pub id: String,
    #[tabled(rename = "名称")]
    pub name: String,
    #[tabled(rename = "应用")]
    pub app: String,
    #[tabled(rename = "状态")]
    pub status: String,
    #[tabled(rename = "Base URL")]
    pub base_url: String,
    #[tabled(rename = "API Key")]
    pub api_key: String,
}

/// 状态行
#[derive(Tabled, Serialize)]
pub struct StatusRow {
    #[tabled(rename = "应用")]
    pub app: String,
    #[tabled(rename = "当前供应商")]
    pub current_provider: String,
    #[tabled(rename = "供应商数量")]
    pub provider_count: String,
    #[tabled(rename = "配置状态")]
    pub config_status: String,
}

/// 路径行
#[derive(Tabled, Serialize)]
pub struct PathRow {
    #[tabled(rename = "应用")]
    pub app: String,
    #[tabled(rename = "配置目录")]
    pub config_dir: String,
    #[tabled(rename = "配置文件")]
    pub settings_file: String,
}

/// 打印表格
#[allow(dead_code)]
pub fn print_table<T: Tabled>(ctx: &OutputContext, data: Vec<T>) {
    match ctx.format {
        OutputFormat::Table => {
            if data.is_empty() {
                println!("{}", "没有数据".dimmed());
                return;
            }
            let table = Table::new(data).with(Style::rounded()).to_string();
            println!("{}", table);
        }
        OutputFormat::Json => {
            // 对于 JSON 输出，需要使用 Serialize
            println!("JSON 输出暂不支持");
        }
        OutputFormat::Yaml => {
            println!("YAML 输出暂不支持");
        }
    }
}

/// 打印供应商列表（支持所有格式）
pub fn print_providers(ctx: &OutputContext, rows: Vec<ProviderRow>) {
    match ctx.format {
        OutputFormat::Table => {
            if rows.is_empty() {
                println!("{}", "没有配置供应商".dimmed());
                return;
            }
            let table = Table::new(&rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&rows).unwrap_or_default();
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&rows).unwrap_or_default();
            println!("{}", yaml);
        }
    }
}

/// 打印状态列表
pub fn print_status(ctx: &OutputContext, rows: Vec<StatusRow>) {
    match ctx.format {
        OutputFormat::Table => {
            if rows.is_empty() {
                println!("{}", "没有配置应用".dimmed());
                return;
            }
            let table = Table::new(&rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&rows).unwrap_or_default();
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&rows).unwrap_or_default();
            println!("{}", yaml);
        }
    }
}

/// 打印路径列表
pub fn print_paths(ctx: &OutputContext, rows: Vec<PathRow>) {
    match ctx.format {
        OutputFormat::Table => {
            let table = Table::new(&rows).with(Style::rounded()).to_string();
            println!("{}", table);
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&rows).unwrap_or_default();
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(&rows).unwrap_or_default();
            println!("{}", yaml);
        }
    }
}

/// 打印成功消息
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message.green());
}

/// 打印错误消息
pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message.red());
}

/// 打印警告消息
pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message.yellow());
}

/// 打印信息消息
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

/// 格式化状态标签
pub fn format_status(is_current: bool) -> String {
    if is_current {
        "● 当前".green().bold().to_string()
    } else {
        "○".dimmed().to_string()
    }
}

/// 截断字符串
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// 脱敏 API Key（显示前缀和后缀）
pub fn mask_api_key(key: &str) -> String {
    if key.is_empty() {
        return "-".to_string();
    }

    let len = key.len();
    if len <= 8 {
        // 太短，全部隐藏
        return format!("{}***", &key[..2.min(len)]);
    }

    // 显示前8个字符和后4个字符
    let prefix_len = 8.min(len);
    let suffix_len = 4.min(len - prefix_len);

    format!(
        "{}...{}",
        &key[..prefix_len],
        &key[len - suffix_len..]
    )
}
