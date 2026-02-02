//! list 命令实现

use anyhow::Result;
use ccswitch_core::{AppState, ProviderService};

use crate::cli::AppTypeArg;
use crate::output::{format_status, print_providers, truncate, OutputContext, ProviderRow};

/// 执行 list 命令
pub fn list_providers(ctx: &OutputContext, app: AppTypeArg, _detail: bool) -> Result<()> {
    let state = AppState::init()?;
    let app_types = app.to_app_types();

    let mut rows: Vec<ProviderRow> = Vec::new();

    for app_type in app_types {
        let providers = ProviderService::list(&state, app_type)?;
        let current_id = ProviderService::current(&state, app_type)?;

        for (id, provider) in providers {
            let is_current = id == current_id;
            let base_url = provider.get_base_url().unwrap_or_else(|| "-".to_string());

            rows.push(ProviderRow {
                id: id.clone(),
                name: provider.name.clone(),
                app: app_type.display_name().to_string(),
                status: format_status(is_current),
                base_url: truncate(&base_url, 40),
            });
        }
    }

    print_providers(ctx, rows);

    Ok(())
}
