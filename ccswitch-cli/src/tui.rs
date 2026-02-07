//! 高级 TUI 界面模块
//!
//! 使用 ratatui 提供现代化的终端用户界面。

use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

use ccswitch_core::{AppState, AppType, ProviderService};

/// 视图类型
#[derive(Clone, Debug, PartialEq)]
enum ViewType {
    /// 主菜单
    MainMenu,
    /// 列表供应商
    ListProviders,
    /// 查看状态
    ViewStatus,
    /// 切换供应商
    SwitchProvider,
    /// 添加默认供应商
    AddDefaultProvider,
    /// 环境冲突管理
    EnvConflictManage,
    /// 消息显示
    MessageBox {
        title: String,
        message: String,
        is_error: bool,
    },
}

/// 默认供应商模板
#[derive(Clone, Debug)]
struct DefaultProviderTemplate {
    /// 供应商名称
    name: String,
    /// 应用类型
    app_type: AppType,
    /// Base URL
    base_url: String,
    /// 默认模型
    default_model: Option<String>,
    /// 描述
    description: String,
}

/// 应用状态
struct App {
    /// 主菜单选项
    menu_items: Vec<MenuItem>,
    /// 当前选中的索引
    selected: usize,
    /// 列表状态
    list_state: ListState,
    /// 是否应该退出
    should_quit: bool,
    /// 状态消息
    status_message: Option<String>,
    /// 当前视图
    current_view: ViewType,
    /// 内容缓冲区（用于子视图）
    content_buffer: Vec<String>,
    /// 供应商列表（用于切换）
    providers: Vec<(String, String)>, // (id, name)
    /// 默认供应商列表（用于快速添加）
    default_providers: Vec<DefaultProviderTemplate>,
    /// 环境冲突操作列表
    env_actions: Vec<EnvAction>,
    /// 应用状态
    app_state: Option<AppState>,
}

/// 环境冲突操作
#[derive(Clone, Debug)]
struct EnvAction {
    title: String,
    description: String,
    action_type: EnvActionType,
}

#[derive(Clone, Debug)]
enum EnvActionType {
    ViewConflicts,
    ClearConflicts,
    BackupConfigs,
    RestoreBackup,
    GenerateUnsetScript,
}

/// 菜单项
#[derive(Clone)]
struct MenuItem {
    title: String,
    description: String,
    key: String,
    action: MenuAction,
}

/// 菜单操作
#[derive(Clone, Debug)]
enum MenuAction {
    ListProviders,
    ViewStatus,
    SwitchProvider,
    AddProvider,
    AddDefaultProvider,
    EditProvider,
    TestProvider,
    RemoveProvider,
    ManageMcp,
    ManagePrompts,
    ManageSkills,
    ProxySettings,
    Speedtest,
    EnvCheck,
    CheckTools,
    ViewConfig,
    CheckUpdate,
    Exit,
}

impl App {
    fn new() -> Result<Self> {
        let app_state = AppState::init().ok();

        let menu_items = vec![
            MenuItem {
                title: "列出供应商".to_string(),
                description: "查看所有供应商配置，包括 Claude、Codex、Gemini 等".to_string(),
                key: "1".to_string(),
                action: MenuAction::ListProviders,
            },
            MenuItem {
                title: "查看状态".to_string(),
                description: "查看当前使用的供应商及其配置信息".to_string(),
                key: "2".to_string(),
                action: MenuAction::ViewStatus,
            },
            MenuItem {
                title: "切换供应商".to_string(),
                description: "切换到其他已配置的供应商".to_string(),
                key: "3".to_string(),
                action: MenuAction::SwitchProvider,
            },
            MenuItem {
                title: "添加供应商".to_string(),
                description: "添加新的供应商配置（当前需要使用 CLI）".to_string(),
                key: "4".to_string(),
                action: MenuAction::AddProvider,
            },
            MenuItem {
                title: "添加官方默认供应商".to_string(),
                description: "快速添加 Anthropic/OpenAI/Google 官方供应商".to_string(),
                key: "4a".to_string(),
                action: MenuAction::AddDefaultProvider,
            },
            MenuItem {
                title: "编辑供应商".to_string(),
                description: "编辑供应商配置（当前需要使用 CLI）".to_string(),
                key: "5".to_string(),
                action: MenuAction::EditProvider,
            },
            MenuItem {
                title: "测试供应商".to_string(),
                description: "测试供应商 API 连接".to_string(),
                key: "6".to_string(),
                action: MenuAction::TestProvider,
            },
            MenuItem {
                title: "删除供应商".to_string(),
                description: "删除供应商配置（当前需要使用 CLI）".to_string(),
                key: "7".to_string(),
                action: MenuAction::RemoveProvider,
            },
            MenuItem {
                title: "MCP 服务器".to_string(),
                description: "管理 MCP 服务器（当前需要使用 CLI）".to_string(),
                key: "8".to_string(),
                action: MenuAction::ManageMcp,
            },
            MenuItem {
                title: "Prompts".to_string(),
                description: "管理系统提示词（当前需要使用 CLI）".to_string(),
                key: "9".to_string(),
                action: MenuAction::ManagePrompts,
            },
            MenuItem {
                title: "Skills".to_string(),
                description: "管理 Skills 扩展（当前需要使用 CLI）".to_string(),
                key: "10".to_string(),
                action: MenuAction::ManageSkills,
            },
            MenuItem {
                title: "代理设置".to_string(),
                description: "设置全局代理（当前需要使用 CLI）".to_string(),
                key: "11".to_string(),
                action: MenuAction::ProxySettings,
            },
            MenuItem {
                title: "端点测速".to_string(),
                description: "测试 API 端点延迟（当前需要使用 CLI）".to_string(),
                key: "12".to_string(),
                action: MenuAction::Speedtest,
            },
            MenuItem {
                title: "环境检测".to_string(),
                description: "检测环境变量冲突".to_string(),
                key: "13".to_string(),
                action: MenuAction::EnvCheck,
            },
            MenuItem {
                title: "工具检测".to_string(),
                description: "检测并安装 AI CLI 工具（Claude Code, Codex, Gemini）".to_string(),
                key: "13a".to_string(),
                action: MenuAction::CheckTools,
            },
            MenuItem {
                title: "查看配置".to_string(),
                description: "查看配置文件路径和内容".to_string(),
                key: "14".to_string(),
                action: MenuAction::ViewConfig,
            },
            MenuItem {
                title: "检测更新".to_string(),
                description: "检测更新/自动更新（当前需要使用 CLI）".to_string(),
                key: "15".to_string(),
                action: MenuAction::CheckUpdate,
            },
            MenuItem {
                title: "退出程序".to_string(),
                description: "退出 CC-Switch TUI".to_string(),
                key: "0".to_string(),
                action: MenuAction::Exit,
            },
        ];

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let default_providers = vec![
            DefaultProviderTemplate {
                name: "Anthropic 官方".to_string(),
                app_type: AppType::Claude,
                base_url: "https://api.anthropic.com".to_string(),
                default_model: Some("claude-sonnet-4-5-20250929".to_string()),
                description: "Anthropic Claude 官方 API".to_string(),
            },
            DefaultProviderTemplate {
                name: "OpenAI 官方".to_string(),
                app_type: AppType::Codex,
                base_url: "https://api.openai.com/v1".to_string(),
                default_model: Some("gpt-4".to_string()),
                description: "OpenAI 官方 API（适用于 Codex CLI）".to_string(),
            },
            DefaultProviderTemplate {
                name: "Google Gemini 官方".to_string(),
                app_type: AppType::Gemini,
                base_url: "https://generativelanguage.googleapis.com".to_string(),
                default_model: Some("gemini-2.0-flash-exp".to_string()),
                description: "Google Gemini 官方 API".to_string(),
            },
        ];

        let env_actions = vec![
            EnvAction {
                title: "查看环境冲突".to_string(),
                description: "检测并显示所有环境变量冲突".to_string(),
                action_type: EnvActionType::ViewConflicts,
            },
            EnvAction {
                title: "清除环境冲突".to_string(),
                description: "从 Shell 配置文件中移除冲突的环境变量".to_string(),
                action_type: EnvActionType::ClearConflicts,
            },
            EnvAction {
                title: "备份配置文件".to_string(),
                description: "备份所有 Shell 配置文件（.bashrc, .zshrc 等）".to_string(),
                action_type: EnvActionType::BackupConfigs,
            },
            EnvAction {
                title: "恢复备份".to_string(),
                description: "从备份中恢复 Shell 配置文件".to_string(),
                action_type: EnvActionType::RestoreBackup,
            },
            EnvAction {
                title: "生成清除脚本".to_string(),
                description: "生成用于清除当前会话环境变量的脚本".to_string(),
                action_type: EnvActionType::GenerateUnsetScript,
            },
        ];

        Ok(Self {
            menu_items,
            selected: 0,
            list_state,
            should_quit: false,
            status_message: None,
            current_view: ViewType::MainMenu,
            content_buffer: Vec::new(),
            providers: Vec::new(),
            default_providers,
            env_actions,
            app_state,
        })
    }

    fn next(&mut self) {
        let len = if self.current_view == ViewType::MainMenu {
            self.menu_items.len()
        } else if matches!(self.current_view, ViewType::SwitchProvider) {
            self.providers.len()
        } else if matches!(self.current_view, ViewType::AddDefaultProvider) {
            self.default_providers.len()
        } else if matches!(self.current_view, ViewType::EnvConflictManage) {
            self.env_actions.len()
        } else {
            return;
        };

        if len == 0 {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= len - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.selected = i;
    }

    fn previous(&mut self) {
        let len = if self.current_view == ViewType::MainMenu {
            self.menu_items.len()
        } else if matches!(self.current_view, ViewType::SwitchProvider) {
            self.providers.len()
        } else if matches!(self.current_view, ViewType::AddDefaultProvider) {
            self.default_providers.len()
        } else if matches!(self.current_view, ViewType::EnvConflictManage) {
            self.env_actions.len()
        } else {
            return;
        };

        if len == 0 {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    len - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.selected = i;
    }

    fn select(&mut self) -> Result<()> {
        match &self.current_view {
            ViewType::MainMenu => {
                if let Some(item) = self.menu_items.get(self.selected) {
                    self.handle_menu_action(item.action.clone())?;
                }
            }
            ViewType::SwitchProvider => {
                self.handle_switch_provider()?;
            }
            ViewType::AddDefaultProvider => {
                self.handle_add_default_provider()?;
            }
            ViewType::EnvConflictManage => {
                self.handle_env_action()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_menu_action(&mut self, action: MenuAction) -> Result<()> {
        match action {
            MenuAction::Exit => {
                self.should_quit = true;
            }
            MenuAction::ListProviders => {
                self.load_providers_list()?;
            }
            MenuAction::ViewStatus => {
                self.load_status_view()?;
            }
            MenuAction::SwitchProvider => {
                self.load_switch_provider_view()?;
            }
            MenuAction::AddDefaultProvider => {
                self.load_add_default_provider_view()?;
            }
            MenuAction::EnvCheck => {
                self.load_env_conflict_manage_view()?;
            }
            MenuAction::CheckTools => {
                self.load_tool_check()?;
            }
            MenuAction::ViewConfig => {
                self.load_config_view()?;
            }
            MenuAction::TestProvider => {
                self.show_message(
                    "测试供应商".to_string(),
                    "此功能需要异步支持，请使用 CLI: cc-switch test".to_string(),
                    false,
                );
            }
            _ => {
                self.show_message(
                    "功能未实现".to_string(),
                    "此功能当前需要使用 CLI 命令行。\n\n按任意键返回主菜单。".to_string(),
                    false,
                );
            }
        }
        Ok(())
    }

    fn load_providers_list(&mut self) -> Result<()> {
        let state = self
            .app_state
            .as_ref()
            .context("应用状态未初始化")?;

        self.content_buffer.clear();
        self.content_buffer.push("═".repeat(60));
        self.content_buffer.push("供应商列表".to_string());
        self.content_buffer.push("═".repeat(60));
        self.content_buffer.push("".to_string());

        for app_type in &[AppType::Claude, AppType::Codex, AppType::Gemini] {
            self.content_buffer.push(format!("【{}】", app_type.display_name()));
            self.content_buffer.push("─".repeat(60));

            match ProviderService::list(state, *app_type) {
                Ok(providers) => {
                    if providers.is_empty() {
                        self.content_buffer.push("  无配置".to_string());
                    } else {
                        // 获取当前供应商
                        let current = ProviderService::current(state, *app_type).ok();

                        for (id, provider) in providers {
                            let is_current = current.as_ref().map(|c| c == &id).unwrap_or(false);
                            let marker = if is_current { "★" } else { " " };
                            self.content_buffer.push(format!(
                                "  {} {} ({})",
                                marker, provider.name, id
                            ));

                            if let Some(url) = provider.get_base_url() {
                                self.content_buffer.push(format!("     URL: {}", url));
                            }
                        }
                    }
                }
                Err(e) => {
                    self.content_buffer.push(format!("  错误: {}", e));
                }
            }
            self.content_buffer.push("".to_string());
        }

        self.content_buffer.push("".to_string());
        self.content_buffer.push("提示: 按 Esc 或 q 返回主菜单".to_string());

        self.current_view = ViewType::ListProviders;
        Ok(())
    }

    fn load_status_view(&mut self) -> Result<()> {
        let state = self
            .app_state
            .as_ref()
            .context("应用状态未初始化")?;

        self.content_buffer.clear();
        self.content_buffer.push("═".repeat(60));
        self.content_buffer.push("当前供应商状态".to_string());
        self.content_buffer.push("═".repeat(60));
        self.content_buffer.push("".to_string());

        for app_type in &[AppType::Claude, AppType::Codex, AppType::Gemini] {
            let current = ProviderService::current(state, *app_type)
                .unwrap_or_else(|_| "未设置".to_string());

            let display_name = app_type.display_name();
            self.content_buffer.push(format!(
                "{:<15} ➜  {}",
                format!("{}:", display_name),
                current
            ));

            // 获取详细信息
            if let Ok(providers) = ProviderService::list(state, *app_type) {
                if let Some((_, provider)) = providers.iter().find(|(id, _)| **id == current) {
                    if let Some(url) = provider.get_base_url() {
                        self.content_buffer.push(format!("                  URL: {}", url));
                    }
                    if let Some(model) = provider.get_model() {
                        self.content_buffer.push(format!("                  Model: {}", model));
                    }
                }
            }
            self.content_buffer.push("".to_string());
        }

        self.content_buffer.push("".to_string());
        self.content_buffer.push("提示: 按 Esc 或 q 返回主菜单".to_string());

        self.current_view = ViewType::ViewStatus;
        Ok(())
    }

    fn load_switch_provider_view(&mut self) -> Result<()> {
        let state = self
            .app_state
            .as_ref()
            .context("应用状态未初始化")?;

        self.providers.clear();
        self.content_buffer.clear();

        // 暂时只支持 Claude
        let app_type = AppType::Claude;

        match ProviderService::list(state, app_type) {
            Ok(providers) => {
                if providers.is_empty() {
                    self.show_message(
                        "无可用供应商".to_string(),
                        "当前没有配置任何供应商。\n请使用 CLI 添加供应商。".to_string(),
                        false,
                    );
                    return Ok(());
                }

                for (id, provider) in providers {
                    self.providers.push((id.clone(), provider.name.clone()));
                    self.content_buffer.push(provider.name);
                }

                self.list_state.select(Some(0));
                self.selected = 0;
                self.current_view = ViewType::SwitchProvider;
            }
            Err(e) => {
                self.show_message(
                    "加载失败".to_string(),
                    format!("无法加载供应商列表: {}", e),
                    true,
                );
            }
        }

        Ok(())
    }

    fn handle_switch_provider(&mut self) -> Result<()> {
        if let Some((id, name)) = self.providers.get(self.selected).cloned() {
            let state = self
                .app_state
                .as_ref()
                .context("应用状态未初始化")?;

            // 暂时只支持 Claude
            let app_type = AppType::Claude;

            match ProviderService::switch(state, app_type, &id) {
                Ok(_) => {
                    self.show_message(
                        "切换成功".to_string(),
                        format!("已切换到供应商: {}", name),
                        false,
                    );
                }
                Err(e) => {
                    self.show_message(
                        "切换失败".to_string(),
                        format!("无法切换供应商: {}", e),
                        true,
                    );
                }
            }
        }

        Ok(())
    }

    fn load_add_default_provider_view(&mut self) -> Result<()> {
        self.content_buffer.clear();
        self.current_view = ViewType::AddDefaultProvider;
        self.selected = 0;
        self.list_state.select(Some(0));
        Ok(())
    }

    fn handle_add_default_provider(&mut self) -> Result<()> {
        if let Some(template) = self.default_providers.get(self.selected).cloned() {
            // 提示用户输入 API Key - 在 TUI 中我们只能显示提示，实际输入需要 CLI
            let message = format!(
                "添加 {} 供应商\n\n\
                应用类型: {}\n\
                Base URL: {}\n\
                默认模型: {}\n\n\
                由于 TUI 限制，请使用以下命令添加:\n\n\
                cc-switch add \"{}\" \\\n  \
                --app {} \\\n  \
                --api-key YOUR_API_KEY \\\n  \
                --base-url {} \\\n  \
                --model {}\n\n\
                按任意键返回",
                template.name,
                template.app_type.display_name(),
                template.base_url,
                template.default_model.as_deref().unwrap_or("默认"),
                template.name,
                match template.app_type {
                    AppType::Claude => "claude",
                    AppType::Codex => "codex",
                    AppType::Gemini => "gemini",
                    _ => "claude",
                },
                template.base_url,
                template.default_model.as_deref().unwrap_or(""),
            );

            self.show_message("添加默认供应商".to_string(), message, false);
        }

        Ok(())
    }

    fn load_env_conflict_manage_view(&mut self) -> Result<()> {
        self.current_view = ViewType::EnvConflictManage;
        self.selected = 0;
        self.list_state.select(Some(0));
        Ok(())
    }

    fn handle_env_action(&mut self) -> Result<()> {
        use ccswitch_core::services::EnvCheckerService;

        if let Some(action) = self.env_actions.get(self.selected).cloned() {
            match action.action_type {
                EnvActionType::ViewConflicts => {
                    self.load_env_check()?;
                }
                EnvActionType::ClearConflicts => {
                    // 先备份
                    match EnvCheckerService::backup_shell_configs() {
                        Ok(backup_path) => {
                            let mut message = format!("已创建备份: {}\n\n", backup_path.display());

                            // 清除所有应用的冲突
                            for app_type in &[AppType::Claude, AppType::Codex, AppType::Gemini] {
                                match EnvCheckerService::remove_env_from_shell_configs(*app_type) {
                                    Ok(files) => {
                                        if !files.is_empty() {
                                            message.push_str(&format!("\n【{}】\n", app_type.display_name()));
                                            message.push_str("已从以下文件中移除环境变量:\n");
                                            for file in files {
                                                message.push_str(&format!("  - {}\n", file));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        message.push_str(&format!("\n{}清除失败: {}\n", app_type.display_name(), e));
                                    }
                                }
                            }

                            message.push_str("\n⚠️ 请重启终端以使更改生效\n\n按任意键返回");
                            self.show_message("清除环境冲突".to_string(), message, false);
                        }
                        Err(e) => {
                            self.show_message(
                                "备份失败".to_string(),
                                format!("无法创建备份: {}\n\n未进行任何清除操作\n按任意键返回", e),
                                true,
                            );
                        }
                    }
                }
                EnvActionType::BackupConfigs => {
                    match EnvCheckerService::backup_shell_configs() {
                        Ok(backup_path) => {
                            self.show_message(
                                "备份成功".to_string(),
                                format!(
                                    "配置文件已备份至:\n{}\n\n按任意键返回",
                                    backup_path.display()
                                ),
                                false,
                            );
                        }
                        Err(e) => {
                            self.show_message(
                                "备份失败".to_string(),
                                format!("无法创建备份: {}\n\n按任意键返回", e),
                                true,
                            );
                        }
                    }
                }
                EnvActionType::RestoreBackup => {
                    match EnvCheckerService::list_backups() {
                        Ok(backups) => {
                            if backups.is_empty() {
                                self.show_message(
                                    "恢复备份".to_string(),
                                    "未找到任何备份\n\n按任意键返回".to_string(),
                                    false,
                                );
                            } else {
                                // 使用最新的备份
                                let latest = &backups[0];
                                match EnvCheckerService::restore_backup(latest) {
                                    Ok(files) => {
                                        let mut message = format!(
                                            "已从备份恢复:\n{}\n\n",
                                            latest.display()
                                        );
                                        message.push_str("恢复的文件:\n");
                                        for file in files {
                                            message.push_str(&format!("  - {}\n", file));
                                        }
                                        message.push_str("\n⚠️ 请重启终端以使更改生效\n\n按任意键返回");

                                        self.show_message("恢复成功".to_string(), message, false);
                                    }
                                    Err(e) => {
                                        self.show_message(
                                            "恢复失败".to_string(),
                                            format!("无法恢复备份: {}\n\n按任意键返回", e),
                                            true,
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            self.show_message(
                                "错误".to_string(),
                                format!("无法列出备份: {}\n\n按任意键返回", e),
                                true,
                            );
                        }
                    }
                }
                EnvActionType::GenerateUnsetScript => {
                    let home = ccswitch_core::config::get_home_dir();
                    let script_path = home.join(".cc-switch-unset.sh");

                    let mut all_script = String::new();
                    all_script.push_str("#!/bin/bash\n");
                    all_script.push_str("# CC-Switch 环境变量清除脚本（所有应用）\n\n");

                    for app_type in &[AppType::Claude, AppType::Codex, AppType::Gemini] {
                        all_script.push_str(&EnvCheckerService::generate_unset_script(*app_type));
                        all_script.push_str("\n");
                    }

                    match std::fs::write(&script_path, all_script) {
                        Ok(_) => {
                            // 设置可执行权限
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                if let Ok(metadata) = std::fs::metadata(&script_path) {
                                    let mut perms = metadata.permissions();
                                    perms.set_mode(0o755);
                                    let _ = std::fs::set_permissions(&script_path, perms);
                                }
                            }

                            self.show_message(
                                "脚本生成成功".to_string(),
                                format!(
                                    "清除脚本已生成:\n{}\n\n\
                                    使用方法:\n\
                                    1. source {}\n\
                                    2. 或重启终端\n\n\
                                    按任意键返回",
                                    script_path.display(),
                                    script_path.display()
                                ),
                                false,
                            );
                        }
                        Err(e) => {
                            self.show_message(
                                "生成失败".to_string(),
                                format!("无法写入脚本文件: {}\n\n按任意键返回", e),
                                true,
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn load_env_check(&mut self) -> Result<()> {
        use ccswitch_core::services::EnvCheckerService;

        let _state = self
            .app_state
            .as_ref()
            .context("应用状态未初始化")?;

        self.content_buffer.clear();
        self.content_buffer.push("═".repeat(60));
        self.content_buffer.push("环境变量检测".to_string());
        self.content_buffer.push("═".repeat(60));
        self.content_buffer.push("".to_string());

        for app_type in &[AppType::Claude, AppType::Codex, AppType::Gemini] {
            self.content_buffer.push(format!("【{}】", app_type.display_name()));
            self.content_buffer.push("─".repeat(60));

            match EnvCheckerService::check(*app_type) {
                Ok(result) => {
                    if result.conflicts.is_empty() {
                        self.content_buffer.push("  ✓ 无冲突".to_string());
                    } else {
                        for conflict in result.conflicts {
                            let value_display = conflict.value.unwrap_or_else(|| "<未设置>".to_string());
                            self.content_buffer.push(format!("  ⚠ {} = {}", conflict.name, value_display));
                        }
                    }
                }
                Err(e) => {
                    self.content_buffer.push(format!("  错误: {}", e));
                }
            }
            self.content_buffer.push("".to_string());
        }

        self.content_buffer.push("".to_string());
        self.content_buffer.push("提示: 按 Esc 或 q 返回主菜单".to_string());

        self.current_view = ViewType::ViewStatus;
        Ok(())
    }

    fn load_tool_check(&mut self) -> Result<()> {
        use std::process::Command;

        self.content_buffer.clear();
        self.content_buffer.push("═".repeat(60));
        self.content_buffer.push("AI CLI 工具检测".to_string());
        self.content_buffer.push("═".repeat(60));
        self.content_buffer.push("".to_string());

        // 定义要检测的工具
        let tools = vec![
            ("Claude Code", "claude-code", "https://github.com/anthropics/claude-code"),
            ("Codex CLI", "codex", "https://developers.openai.com/codex/cli/"),
            ("Gemini CLI", "gemini", "https://github.com/google-gemini/gemini-cli"),
        ];

        for (name, cmd, url) in tools {
            self.content_buffer.push(format!("【{}】", name));
            self.content_buffer.push("─".repeat(60));

            // 检测工具是否安装
            let installed = Command::new("which")
                .arg(cmd)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false);

            if installed {
                // 已安装，尝试获取版本
                if let Ok(output) = Command::new(cmd).arg("--version").output() {
                    if output.status.success() {
                        let version = String::from_utf8_lossy(&output.stdout);
                        let version_line = version.lines().next().unwrap_or("未知版本");
                        self.content_buffer.push(format!("  ✓ 已安装: {}", version_line));
                    } else {
                        self.content_buffer.push("  ✓ 已安装（无法获取版本）".to_string());
                    }
                } else {
                    self.content_buffer.push("  ✓ 已安装".to_string());
                }
            } else {
                self.content_buffer.push("  ✗ 未安装".to_string());
                self.content_buffer.push("".to_string());

                // 根据不同工具提供安装指南
                match cmd {
                    "claude-code" => {
                        self.content_buffer.push("  安装方法:".to_string());
                        self.content_buffer.push("    1. 访问 https://claude.com/claude-code".to_string());
                        self.content_buffer.push("    2. 下载 macOS 版本并安装".to_string());
                        self.content_buffer.push("    3. 或使用 Homebrew:".to_string());
                        self.content_buffer.push("       brew install claude-code".to_string());
                    }
                    "codex" => {
                        self.content_buffer.push("  安装方法:".to_string());
                        self.content_buffer.push("    1. 访问 https://developers.openai.com/codex/cli/".to_string());
                        self.content_buffer.push("    2. 按照官方文档安装 Codex CLI".to_string());
                        self.content_buffer.push("    3. 或使用 npm:".to_string());
                        self.content_buffer.push("       npm install -g @openai/codex-cli".to_string());
                    }
                    "gemini" => {
                        self.content_buffer.push("  安装方法:".to_string());
                        self.content_buffer.push("    1. 访问 https://github.com/google-gemini/gemini-cli".to_string());
                        self.content_buffer.push("    2. 使用 pip 安装:".to_string());
                        self.content_buffer.push("       pip install google-gemini-cli".to_string());
                        self.content_buffer.push("    3. 或使用 Homebrew:".to_string());
                        self.content_buffer.push("       brew install gemini-cli".to_string());
                    }
                    _ => {}
                }
                self.content_buffer.push("".to_string());
                self.content_buffer.push(format!("  官方网站: {}", url));
            }
            self.content_buffer.push("".to_string());
        }

        self.content_buffer.push("".to_string());
        self.content_buffer.push("提示: 按 Esc 或 q 返回主菜单".to_string());

        self.current_view = ViewType::ViewStatus;
        Ok(())
    }

    fn load_config_view(&mut self) -> Result<()> {
        use ccswitch_core::config;

        self.content_buffer.clear();
        self.content_buffer.push("═".repeat(60));
        self.content_buffer.push("配置文件路径".to_string());
        self.content_buffer.push("═".repeat(60));
        self.content_buffer.push("".to_string());

        // 显示配置路径
        let data_dir = config::get_app_config_dir();
        self.content_buffer.push(format!("应用目录: {}", data_dir.display()));
        self.content_buffer.push("".to_string());

        // 显示各个应用的配置目录
        self.content_buffer.push(format!(
            "Claude Code: {}",
            config::get_claude_config_dir().display()
        ));
        self.content_buffer.push(format!(
            "Codex CLI: {}",
            config::get_codex_config_dir().display()
        ));
        self.content_buffer.push(format!(
            "Gemini CLI: {}",
            config::get_gemini_config_dir().display()
        ));

        self.content_buffer.push("".to_string());
        self.content_buffer.push("提示: 按 Esc 或 q 返回主菜单".to_string());

        self.current_view = ViewType::ViewStatus;
        Ok(())
    }

    fn show_message(&mut self, title: String, message: String, is_error: bool) {
        self.content_buffer.clear();
        self.content_buffer.push(title.clone());
        self.content_buffer.push("".to_string());
        self.content_buffer.extend(message.lines().map(|s| s.to_string()));

        self.current_view = ViewType::MessageBox {
            title,
            message,
            is_error,
        };
    }

    fn back_to_main_menu(&mut self) {
        self.current_view = ViewType::MainMenu;
        self.content_buffer.clear();
        self.providers.clear();
        self.list_state.select(Some(self.selected));
    }
}

/// 运行 TUI 应用
pub fn run_tui() -> Result<()> {
    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建应用状态
    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app);

    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("错误: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.current_view {
                        ViewType::MainMenu => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                app.should_quit = true;
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.next();
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.previous();
                            }
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                app.select()?;
                            }
                            KeyCode::Char(c) if c.is_ascii_digit() => {
                                let num = c.to_string();
                                if let Some(pos) =
                                    app.menu_items.iter().position(|item| item.key == num)
                                {
                                    app.selected = pos;
                                    app.list_state.select(Some(pos));
                                    app.select()?;
                                }
                            }
                            _ => {}
                        },
                        ViewType::SwitchProvider => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                app.back_to_main_menu();
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.next();
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.previous();
                            }
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                app.select()?;
                            }
                            _ => {}
                        },
                        _ => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                                app.back_to_main_menu();
                            }
                            _ => {}
                        },
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    match &app.current_view {
        ViewType::MainMenu => render_main_view(f, app, size),
        ViewType::ListProviders | ViewType::ViewStatus => render_content_view(f, app, size),
        ViewType::SwitchProvider => render_switch_provider_view(f, app, size),
        ViewType::AddDefaultProvider => render_add_default_provider_view(f, app, size),
        ViewType::EnvConflictManage => render_env_conflict_manage_view(f, app, size),
        ViewType::MessageBox { title, message, is_error } => {
            render_message_box(f, size, title, message, *is_error)
        }
    }
}

fn render_main_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 标题
            Constraint::Min(0),     // 主内容
            Constraint::Length(3),  // 状态栏
        ])
        .split(area);

    render_header(f, chunks[0]);

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    render_menu(f, app, content_chunks[0]);
    render_description(f, app, content_chunks[1]);
    render_footer(f, app, chunks[2]);
}

fn render_content_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 标题
            Constraint::Min(0),     // 内容
            Constraint::Length(3),  // 状态栏
        ])
        .split(area);

    render_header(f, chunks[0]);

    let content: Vec<Line> = app
        .content_buffer
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect();

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" 📄 详细信息 ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, chunks[1]);

    let footer_text = vec![Line::from(vec![
        Span::styled("按 ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 或 ", Style::default().fg(Color::DarkGray)),
        Span::styled("q", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 返回主菜单", Style::default().fg(Color::DarkGray)),
    ])];

    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        )
        .alignment(Alignment::Center);

    f.render_widget(footer, chunks[2]);
}

fn render_switch_provider_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 标题
            Constraint::Min(0),     // 列表
            Constraint::Length(3),  // 状态栏
        ])
        .split(area);

    render_header(f, chunks[0]);

    let items: Vec<ListItem> = app
        .providers
        .iter()
        .enumerate()
        .map(|(i, (_, name))| {
            let style = if i == app.selected {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            ListItem::new(Line::from(Span::styled(name, style)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" 🔄 选择供应商 (Claude) ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], &mut app.list_state.clone());

    let footer_text = vec![Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 选择  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 确认  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 取消", Style::default().fg(Color::DarkGray)),
    ])];

    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        )
        .alignment(Alignment::Center);

    f.render_widget(footer, chunks[2]);
}

fn render_add_default_provider_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 标题
            Constraint::Min(0),     // 列表
            Constraint::Length(3),  // 状态栏
        ])
        .split(area);

    render_header(f, chunks[0]);

    let items: Vec<ListItem> = app
        .default_providers
        .iter()
        .enumerate()
        .map(|(i, template)| {
            let style = if i == app.selected {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let icon = match template.app_type {
                AppType::Claude => "🤖",
                AppType::Codex => "🔧",
                AppType::Gemini => "✨",
                _ => "📦",
            };

            let content = vec![
                Line::from(vec![
                    Span::styled(format!("{} ", icon), style),
                    Span::styled(&template.name, style),
                ]),
                Line::from(vec![
                    Span::styled("    ", style),
                    Span::styled(&template.description, Style::default().fg(Color::DarkGray)),
                ]),
            ];

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" ⚡ 添加官方默认供应商 ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], &mut app.list_state.clone());

    let footer_text = vec![Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 选择  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 查看命令  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 返回", Style::default().fg(Color::DarkGray)),
    ])];

    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        )
        .alignment(Alignment::Center);

    f.render_widget(footer, chunks[2]);
}

fn render_env_conflict_manage_view(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 标题
            Constraint::Min(0),     // 列表
            Constraint::Length(3),  // 状态栏
        ])
        .split(area);

    render_header(f, chunks[0]);

    let items: Vec<ListItem> = app
        .env_actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let style = if i == app.selected {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let icon = match action.action_type {
                EnvActionType::ViewConflicts => "🔍",
                EnvActionType::ClearConflicts => "🧹",
                EnvActionType::BackupConfigs => "💾",
                EnvActionType::RestoreBackup => "↩️",
                EnvActionType::GenerateUnsetScript => "📝",
            };

            let content = vec![
                Line::from(vec![
                    Span::styled(format!("{} ", icon), style),
                    Span::styled(&action.title, style),
                ]),
                Line::from(vec![
                    Span::styled("    ", style),
                    Span::styled(&action.description, Style::default().fg(Color::DarkGray)),
                ]),
            ];

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" ⚠️  环境冲突管理 ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, chunks[1], &mut app.list_state.clone());

    let footer_text = vec![Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 选择  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 执行  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Esc", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 返回", Style::default().fg(Color::DarkGray)),
    ])];

    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        )
        .alignment(Alignment::Center);

    f.render_widget(footer, chunks[2]);
}

fn render_message_box(f: &mut Frame, area: Rect, title: &str, message: &str, is_error: bool) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 标题
            Constraint::Min(0),     // 消息
            Constraint::Length(3),  // 提示
        ])
        .split(area);

    render_header(f, chunks[0]);

    let (border_color, title_icon) = if is_error {
        (Color::Red, "❌ ")
    } else {
        (Color::Green, "✓ ")
    };

    let content: Vec<Line> = message.lines().map(|s| Line::from(s)).collect();

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(format!(" {}{} ", title_icon, title))
                .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
        )
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Center);

    f.render_widget(paragraph, chunks[1]);

    let footer_text = vec![Line::from(vec![
        Span::styled("按 ", Style::default().fg(Color::DarkGray)),
        Span::styled("任意键", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" 返回主菜单", Style::default().fg(Color::DarkGray)),
    ])];

    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
        )
        .alignment(Alignment::Center);

    f.render_widget(footer, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect) {
    let title = vec![Line::from(vec![
        Span::styled(
            " ⚡ CC-Switch ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "AI CLI 配置管理器 ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("v{} ", ccswitch_core::VERSION),
            Style::default().fg(Color::DarkGray),
        ),
    ])];

    let header = Paragraph::new(title)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .alignment(Alignment::Center);

    f.render_widget(header, area);
}

fn render_menu(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let key_style = if i == app.selected {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let title_style = if i == app.selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let content = Line::from(vec![
                Span::styled(format!("{:>2}. ", item.key), key_style),
                Span::styled(&item.title, title_style),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" 📋 主菜单 ")
                .title_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut app.list_state.clone());
}

fn render_description(f: &mut Frame, app: &App, area: Rect) {
    let description = if let Some(item) = app.menu_items.get(app.selected) {
        vec![
            Line::from(vec![
                Span::styled(
                    "功能: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(&item.title, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "说明: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(Span::styled(
                &item.description,
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(vec![Span::styled(
                "快捷键: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  ↑/k  ", Style::default().fg(Color::Green)),
                Span::raw("- 上移"),
            ]),
            Line::from(vec![
                Span::styled("  ↓/j  ", Style::default().fg(Color::Green)),
                Span::raw("- 下移"),
            ]),
            Line::from(vec![
                Span::styled("  Enter", Style::default().fg(Color::Green)),
                Span::raw(" - 确认选择"),
            ]),
            Line::from(vec![
                Span::styled("  0-15 ", Style::default().fg(Color::Green)),
                Span::raw("- 数字快捷键"),
            ]),
            Line::from(vec![
                Span::styled("  q/Esc", Style::default().fg(Color::Green)),
                Span::raw(" - 退出程序"),
            ]),
        ]
    } else {
        vec![Line::from("请选择一个菜单项")]
    };

    let paragraph = Paragraph::new(description)
        .block(
            Block::default()
                .title(" 📝 详细信息 ")
                .title_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let status = if let Some(msg) = &app.status_message {
        vec![Line::from(vec![
            Span::styled("⚡ 状态: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(msg, Style::default().fg(Color::Green)),
        ])]
    } else {
        vec![Line::from(vec![
            Span::styled("✓ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("就绪", Style::default().fg(Color::Green)),
            Span::raw(" │ "),
            Span::styled(
                "↑↓/j/k",
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" 导航", Style::default().fg(Color::DarkGray)),
            Span::raw(" │ "),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::styled(" 确认", Style::default().fg(Color::DarkGray)),
            Span::raw(" │ "),
            Span::styled("q/Esc", Style::default().fg(Color::Cyan)),
            Span::styled(" 退出", Style::default().fg(Color::DarkGray)),
        ])]
    };

    let footer = Paragraph::new(status)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left);

    f.render_widget(footer, area);
}

/// 检查当前供应商状态并显示
#[allow(dead_code)]
pub fn show_status_tui() -> Result<()> {
    let state = AppState::init()?;

    let claude_current =
        ProviderService::current(&state, AppType::Claude).unwrap_or_else(|_| "未设置".to_string());
    let codex_current =
        ProviderService::current(&state, AppType::Codex).unwrap_or_else(|_| "未设置".to_string());
    let gemini_current =
        ProviderService::current(&state, AppType::Gemini).unwrap_or_else(|_| "未设置".to_string());

    println!("当前供应商状态:");
    println!("  Claude Code: {}", claude_current);
    println!("  Codex:       {}", codex_current);
    println!("  Gemini CLI:  {}", gemini_current);

    Ok(())
}
