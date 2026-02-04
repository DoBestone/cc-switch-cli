//! 高级 TUI 界面模块
//!
//! 使用 ratatui 提供现代化的终端用户界面。

use anyhow::Result;
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

use ccswitch_core::AppState;

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
}

/// 菜单项
#[derive(Clone)]
struct MenuItem {
    title: String,
    description: String,
    key: String,
}

impl App {
    fn new() -> Self {
        let menu_items = vec![
            MenuItem {
                title: "列出供应商".to_string(),
                description: "查看所有供应商配置".to_string(),
                key: "1".to_string(),
            },
            MenuItem {
                title: "查看状态".to_string(),
                description: "查看当前使用的供应商".to_string(),
                key: "2".to_string(),
            },
            MenuItem {
                title: "切换供应商".to_string(),
                description: "切换到其他供应商".to_string(),
                key: "3".to_string(),
            },
            MenuItem {
                title: "添加供应商".to_string(),
                description: "添加新的供应商配置".to_string(),
                key: "4".to_string(),
            },
            MenuItem {
                title: "编辑供应商".to_string(),
                description: "编辑供应商配置".to_string(),
                key: "5".to_string(),
            },
            MenuItem {
                title: "测试供应商".to_string(),
                description: "测试供应商 API".to_string(),
                key: "6".to_string(),
            },
            MenuItem {
                title: "删除供应商".to_string(),
                description: "删除供应商配置".to_string(),
                key: "7".to_string(),
            },
            MenuItem {
                title: "MCP 服务器".to_string(),
                description: "管理 MCP 服务器".to_string(),
                key: "8".to_string(),
            },
            MenuItem {
                title: "Prompts".to_string(),
                description: "管理系统提示词".to_string(),
                key: "9".to_string(),
            },
            MenuItem {
                title: "Skills".to_string(),
                description: "管理 Skills 扩展".to_string(),
                key: "10".to_string(),
            },
            MenuItem {
                title: "代理设置".to_string(),
                description: "设置全局代理".to_string(),
                key: "11".to_string(),
            },
            MenuItem {
                title: "端点测速".to_string(),
                description: "测试 API 端点延迟".to_string(),
                key: "12".to_string(),
            },
            MenuItem {
                title: "环境检测".to_string(),
                description: "检测环境变量冲突".to_string(),
                key: "13".to_string(),
            },
            MenuItem {
                title: "查看配置".to_string(),
                description: "查看配置文件路径".to_string(),
                key: "14".to_string(),
            },
            MenuItem {
                title: "检测更新".to_string(),
                description: "检测更新/自动更新".to_string(),
                key: "15".to_string(),
            },
            MenuItem {
                title: "退出程序".to_string(),
                description: "退出 CC-Switch".to_string(),
                key: "0".to_string(),
            },
        ];

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            menu_items,
            selected: 0,
            list_state,
            should_quit: false,
            status_message: None,
        }
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.menu_items.len() - 1 {
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
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.menu_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
        self.selected = i;
    }

    fn select(&mut self) {
        if let Some(item) = self.menu_items.get(self.selected) {
            // 根据选择的菜单项执行操作
            match item.key.as_str() {
                "0" => {
                    self.should_quit = true;
                }
                _ => {
                    // 暂时显示一个状态消息
                    self.status_message = Some(format!("选择了: {}", item.title));
                }
            }
        }
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
    let mut app = App::new();
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
        println!("错误: {:?}", err);
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
                    match key.code {
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
                            app.select();
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() => {
                            // 支持数字快捷键
                            let num = c.to_string();
                            if let Some(pos) = app.menu_items.iter().position(|item| item.key == num) {
                                app.selected = pos;
                                app.list_state.select(Some(pos));
                                app.select();
                            }
                        }
                        _ => {}
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

    // 创建主布局
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // 标题
            Constraint::Min(0),     // 主内容
            Constraint::Length(3),  // 状态栏
        ])
        .split(size);

    // 渲染标题
    render_header(f, chunks[0]);

    // 创建内容布局（左侧菜单 + 右侧描述）
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // 渲染菜单列表
    render_menu(f, app, content_chunks[0]);

    // 渲染描述面板
    render_description(f, app, content_chunks[1]);

    // 渲染状态栏
    render_footer(f, app, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect) {
    let title = vec![
        Line::from(vec![
            Span::styled("CC-Switch", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" - "),
            Span::styled("AI CLI 配置管理器", Style::default().fg(Color::White)),
        ]),
    ];

    let header = Paragraph::new(title)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
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
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let title_style = if i == app.selected {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
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
                .title(" 主菜单 ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD)
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut app.list_state.clone());
}

fn render_description(f: &mut Frame, app: &App, area: Rect) {
    let description = if let Some(item) = app.menu_items.get(app.selected) {
        vec![
            Line::from(vec![
                Span::styled("功能: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(&item.title, Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("说明: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(Span::styled(&item.description, Style::default().fg(Color::Gray))),
            Line::from(""),
            Line::from(""),
            Line::from(vec![
                Span::styled("快捷键: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
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
                .title(" 详细信息 ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let status = if let Some(msg) = &app.status_message {
        vec![Line::from(vec![
            Span::styled("状态: ", Style::default().fg(Color::Yellow)),
            Span::styled(msg, Style::default().fg(Color::Green)),
        ])]
    } else {
        vec![Line::from(vec![
            Span::styled("就绪", Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled("使用 ↑↓ 或 j/k 导航", Style::default().fg(Color::DarkGray)),
            Span::raw(" | "),
            Span::styled("Enter 确认", Style::default().fg(Color::DarkGray)),
            Span::raw(" | "),
            Span::styled("q 退出", Style::default().fg(Color::DarkGray)),
        ])]
    };

    let footer = Paragraph::new(status)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White))
        )
        .alignment(Alignment::Left);

    f.render_widget(footer, area);
}

/// 检查当前供应商状态并显示
#[allow(dead_code)]
pub fn show_status_tui() -> Result<()> {
    let state = AppState::init()?;

    // 获取所有应用的当前供应商
    let claude_current = ccswitch_core::ProviderService::current(&state, ccswitch_core::AppType::Claude)
        .unwrap_or_else(|_| "未设置".to_string());
    let codex_current = ccswitch_core::ProviderService::current(&state, ccswitch_core::AppType::Codex)
        .unwrap_or_else(|_| "未设置".to_string());
    let gemini_current = ccswitch_core::ProviderService::current(&state, ccswitch_core::AppType::Gemini)
        .unwrap_or_else(|_| "未设置".to_string());

    println!("当前供应商状态:");
    println!("  Claude Code: {}", claude_current);
    println!("  Codex:       {}", codex_current);
    println!("  Gemini CLI:  {}", gemini_current);

    Ok(())
}
