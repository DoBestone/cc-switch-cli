//! config 命令实现

use anyhow::Result;
use ccswitch_core::ConfigService;

use crate::cli::{AppTypeArg, ConfigAction, ExportFormatArg};
use crate::output::{print_info, print_paths, print_warning, OutputContext, PathRow};

/// 执行 config 子命令
pub fn execute(ctx: &OutputContext, action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Path { app } => show_paths(ctx, app),
        ConfigAction::Open { app } => open_config(app),
        ConfigAction::Check { app } => check_config(ctx, app),
    }
}

/// 显示配置路径
pub fn show_paths(ctx: &OutputContext, app: AppTypeArg) -> Result<()> {
    let app_types = app.to_app_types();
    let mut rows: Vec<PathRow> = Vec::new();

    // 显示 cc-switch 自身配置
    let cc_paths = ConfigService::get_paths();
    rows.push(PathRow {
        app: "cc-switch".to_string(),
        config_dir: cc_paths.app_config_dir.display().to_string(),
        settings_file: cc_paths.settings_path.display().to_string(),
    });

    // 显示各应用配置
    for app_type in app_types {
        let paths = ConfigService::get_app_paths(app_type);
        rows.push(PathRow {
            app: app_type.display_name().to_string(),
            config_dir: paths.config_dir.display().to_string(),
            settings_file: paths.settings_path.display().to_string(),
        });
    }

    print_paths(ctx, rows);

    Ok(())
}

/// 打开配置目录（在终端环境下打印路径）
fn open_config(app: Option<AppTypeArg>) -> Result<()> {
    match app {
        Some(app_arg) => {
            let app_types = app_arg.to_app_types();
            for app_type in app_types {
                let paths = ConfigService::get_app_paths(app_type);
                print_info(&format!(
                    "{}: {}",
                    app_type.display_name(),
                    paths.config_dir.display()
                ));
            }
        }
        None => {
            let paths = ConfigService::get_paths();
            print_info(&format!("cc-switch: {}", paths.app_config_dir.display()));
        }
    }

    print_info("提示: 在终端中使用 'cd <path>' 进入目录");

    Ok(())
}

/// 检查配置状态
fn check_config(ctx: &OutputContext, app: AppTypeArg) -> Result<()> {
    let app_types = app.to_app_types();

    for app_type in app_types {
        let is_configured = ConfigService::is_app_configured(app_type);
        let status = if is_configured { "✓ 已配置" } else { "✗ 未配置" };

        if ctx.format == crate::cli::OutputFormat::Table {
            println!("{}: {}", app_type.display_name(), status);
        }
    }

    Ok(())
}

/// 导出配置
pub fn export(
    _ctx: &OutputContext,
    _format: ExportFormatArg,
    _out: Option<String>,
    _app: AppTypeArg,
) -> Result<()> {
    print_warning("导出功能尚未实现");
    Ok(())
}

/// 导入配置
pub fn import(_ctx: &OutputContext, _file: &str, _app: Option<AppTypeArg>) -> Result<()> {
    print_warning("导入功能尚未实现");
    Ok(())
}
