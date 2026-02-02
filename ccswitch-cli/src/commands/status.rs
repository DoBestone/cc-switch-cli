//! status 命令实现

use anyhow::Result;
use ccswitch_core::{AppState, ProviderService};

use crate::cli::AppTypeArg;
use crate::output::{print_status, OutputContext, StatusRow};

/// 执行 status 命令
pub fn show_status(ctx: &OutputContext, app: AppTypeArg) -> Result<()> {
    let state = AppState::init()?;
    let app_types = app.to_app_types();

    let mut rows: Vec<StatusRow> = Vec::new();

    for app_type in app_types {
        let providers = ProviderService::list(&state, app_type)?;
        let current_id = ProviderService::current(&state, app_type)?;

        let current_name = if current_id.is_empty() {
            "未设置".to_string()
        } else {
            providers
                .get(&current_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| current_id.clone())
        };

        let provider_count = providers.len();
        let config_status = if provider_count > 0 {
            "已配置".to_string()
        } else {
            "未配置".to_string()
        };

        rows.push(StatusRow {
            app: app_type.display_name().to_string(),
            current_provider: current_name,
            provider_count: provider_count.to_string(),
            config_status,
        });
    }

    print_status(ctx, rows);

    Ok(())
}
