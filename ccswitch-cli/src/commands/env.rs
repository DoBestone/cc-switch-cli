//! 环境变量检测命令模块
//!
//! 实现环境变量冲突检测的 CLI 命令。

use anyhow::Result;
use ccswitch_core::EnvCheckerService;

use crate::cli::AppTypeArg;
use crate::output::{print_info, print_success, print_warning, OutputContext};

/// 检查环境变量冲突
pub fn check(_ctx: &OutputContext, app: AppTypeArg) -> Result<()> {
    let app_types = app.to_app_types();

    println!("\n🔍 环境变量冲突检测\n");

    let mut total_conflicts = 0;

    for app_type in app_types {
        let result = EnvCheckerService::check(app_type)?;

        if result.conflicts.is_empty() {
            print_success(&format!("{}: 无冲突", app_type.display_name()));
        } else {
            print_warning(&format!(
                "{}: 发现 {} 个潜在冲突",
                app_type.display_name(),
                result.conflicts.len()
            ));

            for conflict in &result.conflicts {
                let source = match &conflict.source {
                    ccswitch_core::services::env_checker::EnvSource::Process => "进程环境".to_string(),
                    ccswitch_core::services::env_checker::EnvSource::ShellConfig(file) => {
                        format!("Shell 配置 ({})", file)
                    }
                };

                println!("  - {}", conflict.name);
                println!("    来源: {}", source);
                if let Some(value) = &conflict.value {
                    println!("    值: {}", value);
                }
                println!("    说明: {}", conflict.description);
            }

            total_conflicts += result.conflicts.len();
        }
    }

    println!();

    if total_conflicts == 0 {
        print_success("未发现环境变量冲突");
    } else {
        print_warning(&format!("共发现 {} 个潜在冲突", total_conflicts));
        print_info("这些环境变量可能会覆盖配置文件中的设置");
    }

    Ok(())
}

/// 列出相关环境变量
pub fn list(_ctx: &OutputContext, app: AppTypeArg) -> Result<()> {
    let app_types = app.to_app_types();

    println!("\n📋 相关环境变量\n");

    for app_type in app_types {
        println!("{}", app_type.display_name());
        println!("{}", "-".repeat(40));

        let vars = EnvCheckerService::list_env_vars(app_type);

        for (name, value) in vars {
            let value_str = value.unwrap_or_else(|| "(未设置)".to_string());
            println!("  {}: {}", name, value_str);
        }

        println!();
    }

    Ok(())
}