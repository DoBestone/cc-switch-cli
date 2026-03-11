//! 故障转移服务
//!
//! 管理供应商的故障转移队列，支持自动切换到备用供应商。

use crate::app_config::AppType;
use crate::error::AppError;
use crate::provider::Provider;
use crate::store::AppState;
use indexmap::IndexMap;

/// 故障转移队列项
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FailoverQueueItem {
    pub provider_id: String,
    pub provider_name: String,
    pub sort_index: usize,
}

/// 故障转移服务
pub struct FailoverService;

impl FailoverService {
    /// 获取故障转移队列
    pub fn get_queue(state: &AppState, app_type: AppType) -> Result<Vec<FailoverQueueItem>, AppError> {
        let providers = state.db.get_all_providers(app_type.as_str())?;

        let mut queue: Vec<FailoverQueueItem> = providers
            .iter()
            .filter(|(_, p)| p.in_failover_queue)
            .map(|(id, p)| FailoverQueueItem {
                provider_id: id.clone(),
                provider_name: p.name.clone(),
                sort_index: p.sort_index.unwrap_or(0),
            })
            .collect();

        // 按排序索引排序
        queue.sort_by_key(|item| item.sort_index);

        Ok(queue)
    }

    /// 获取可添加到队列的供应商（不在队列中的）
    pub fn get_available_providers(
        state: &AppState,
        app_type: AppType,
    ) -> Result<IndexMap<String, Provider>, AppError> {
        let providers = state.db.get_all_providers(app_type.as_str())?;

        let available: IndexMap<String, Provider> = providers
            .into_iter()
            .filter(|(_, p)| !p.in_failover_queue)
            .collect();

        Ok(available)
    }

    /// 添加供应商到队列
    pub fn add_to_queue(
        state: &AppState,
        app_type: AppType,
        provider_id: &str,
    ) -> Result<(), AppError> {
        let mut providers = state.db.get_all_providers(app_type.as_str())?;

        let provider = providers
            .get_mut(provider_id)
            .ok_or_else(|| AppError::ProviderNotFound(provider_id.to_string()))?;

        provider.in_failover_queue = true;
        state.db.save_provider(app_type.as_str(), provider)?;

        Ok(())
    }

    /// 从队列移除供应商
    pub fn remove_from_queue(
        state: &AppState,
        app_type: AppType,
        provider_id: &str,
    ) -> Result<(), AppError> {
        let mut providers = state.db.get_all_providers(app_type.as_str())?;

        let provider = providers
            .get_mut(provider_id)
            .ok_or_else(|| AppError::ProviderNotFound(provider_id.to_string()))?;

        provider.in_failover_queue = false;
        state.db.save_provider(app_type.as_str(), provider)?;

        Ok(())
    }

    /// 重排队列顺序
    pub fn reorder_queue(
        state: &AppState,
        app_type: AppType,
        provider_ids: &[String],
    ) -> Result<(), AppError> {
        let mut providers = state.db.get_all_providers(app_type.as_str())?;

        for (index, id) in provider_ids.iter().enumerate() {
            if let Some(provider) = providers.get_mut(id) {
                provider.in_failover_queue = true;
                provider.sort_index = Some(index);
                state.db.save_provider(app_type.as_str(), provider)?;
            }
        }

        Ok(())
    }

    /// 清空队列
    pub fn clear_queue(state: &AppState, app_type: AppType) -> Result<(), AppError> {
        let mut providers = state.db.get_all_providers(app_type.as_str())?;

        for provider in providers.values_mut() {
            if provider.in_failover_queue {
                provider.in_failover_queue = false;
                state.db.save_provider(app_type.as_str(), provider)?;
            }
        }

        Ok(())
    }

    /// 获取队列中的下一个供应商
    pub fn get_next_in_queue(
        state: &AppState,
        app_type: AppType,
        current_id: &str,
    ) -> Result<Option<Provider>, AppError> {
        let queue = Self::get_queue(state, app_type.clone())?;

        // 找到当前供应商在队列中的位置
        let current_index = queue
            .iter()
            .position(|item| item.provider_id == current_id);

        if let Some(index) = current_index {
            // 返回下一个供应商
            if index + 1 < queue.len() {
                let next_id = &queue[index + 1].provider_id;
                let providers = state.db.get_all_providers(app_type.as_str())?;
                return Ok(providers.get(next_id).cloned());
            }
        }

        Ok(None)
    }

    /// 故障转移到下一个供应商
    pub fn failover(
        state: &AppState,
        app_type: AppType,
        current_id: &str,
    ) -> Result<Option<String>, AppError> {
        if let Some(next_provider) = Self::get_next_in_queue(state, app_type.clone(), current_id)? {
            // 切换到下一个供应商
            crate::services::ProviderService::switch(state, app_type.clone(), &next_provider.id)?;
            return Ok(Some(next_provider.id));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_failover_queue() {
        let state = AppState::memory().unwrap();

        // 创建测试供应商
        let p1 = Provider::new("p1", "Provider 1", json!({}));
        let p2 = Provider::new("p2", "Provider 2", json!({}));

        state.db.save_provider("claude", &p1).unwrap();
        state.db.save_provider("claude", &p2).unwrap();

        // 添加到队列
        FailoverService::add_to_queue(&state, AppType::Claude, "p1").unwrap();
        FailoverService::add_to_queue(&state, AppType::Claude, "p2").unwrap();

        // 验证队列
        let queue = FailoverService::get_queue(&state, AppType::Claude).unwrap();
        assert_eq!(queue.len(), 2);

        // 移除一个
        FailoverService::remove_from_queue(&state, AppType::Claude, "p1").unwrap();
        let queue = FailoverService::get_queue(&state, AppType::Claude).unwrap();
        assert_eq!(queue.len(), 1);
    }
}