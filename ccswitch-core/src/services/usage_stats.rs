//! 使用统计服务
//!
//! 跟踪 API 使用量，提供使用量报告和限额检查。

use crate::error::AppError;
use crate::store::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// 使用量汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_cost: f64,
    pub period_start: Option<i64>,
    pub period_end: Option<i64>,
}

/// 每日统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStats {
    pub date: String,
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost: f64,
}

/// 供应商统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStats {
    pub provider_id: String,
    pub provider_name: String,
    pub requests: u64,
    pub tokens: u64,
    pub cost: f64,
}

/// 模型统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStats {
    pub model_id: String,
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost: f64,
}

/// 使用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRecord {
    pub id: String,
    pub provider_id: String,
    pub provider_name: String,
    pub app_type: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost: f64,
    pub request_time: i64,
    pub latency_ms: u64,
}

/// 限额状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitStatus {
    pub provider_id: String,
    pub daily_limit: Option<f64>,
    pub daily_used: f64,
    pub monthly_limit: Option<f64>,
    pub monthly_used: f64,
    pub is_exceeded: bool,
}

/// 使用统计服务
pub struct UsageStatsService;

impl UsageStatsService {
    /// 记录使用量
    pub fn record_usage(
        state: &AppState,
        record: &UsageRecord,
    ) -> Result<(), AppError> {
        let record_str = serde_json::to_string(record)
            .map_err(|e| AppError::Config(format!("序列化记录失败: {}", e)))?;

        let key = format!("usage_record_{}", record.id);
        state.db.set_setting(&key, &record_str)?;

        // 更新汇总
        Self::update_summary(state, record)?;

        Ok(())
    }

    fn update_summary(state: &AppState, record: &UsageRecord) -> Result<(), AppError> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let daily_key = format!("usage_daily_{}", today);

        // 获取今日统计
        let daily_stats: DailyStats = if let Some(s) = state.db.get_setting(&daily_key)? {
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            DailyStats {
                date: today.clone(),
                requests: 0,
                input_tokens: 0,
                output_tokens: 0,
                cost: 0.0,
            }
        };

        // 更新统计
        let updated = DailyStats {
            date: today,
            requests: daily_stats.requests + 1,
            input_tokens: daily_stats.input_tokens + record.input_tokens,
            output_tokens: daily_stats.output_tokens + record.output_tokens,
            cost: daily_stats.cost + record.cost,
        };

        let stats_str = serde_json::to_string(&updated)
            .map_err(|e| AppError::Config(format!("序列化统计失败: {}", e)))?;

        state.db.set_setting(&daily_key, &stats_str)?;

        // 更新供应商统计
        let provider_key = format!("usage_provider_{}", record.provider_id);
        let mut provider_stats: ProviderStats = if let Some(s) = state.db.get_setting(&provider_key)? {
            serde_json::from_str(&s).unwrap_or_else(|_| ProviderStats {
                provider_id: record.provider_id.clone(),
                provider_name: record.provider_name.clone(),
                requests: 0,
                tokens: 0,
                cost: 0.0,
            })
        } else {
            ProviderStats {
                provider_id: record.provider_id.clone(),
                provider_name: record.provider_name.clone(),
                requests: 0,
                tokens: 0,
                cost: 0.0,
            }
        };

        provider_stats.requests += 1;
        provider_stats.tokens += record.input_tokens + record.output_tokens;
        provider_stats.cost += record.cost;

        let stats_str = serde_json::to_string(&provider_stats)
            .map_err(|e| AppError::Config(format!("序列化统计失败: {}", e)))?;

        state.db.set_setting(&provider_key, &stats_str)?;

        Ok(())
    }

    /// 获取汇总
    pub fn get_summary(
        state: &AppState,
        start_date: Option<i64>,
        end_date: Option<i64>,
    ) -> Result<UsageSummary, AppError> {
        let mut summary = UsageSummary {
            total_requests: 0,
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            total_cost: 0.0,
            period_start: start_date,
            period_end: end_date,
        };

        // 从设置中获取汇总数据
        if let Some(s) = state.db.get_setting("usage_summary")? {
            if let Ok(stored) = serde_json::from_str::<UsageSummary>(&s) {
                summary = stored;
                summary.period_start = start_date;
                summary.period_end = end_date;
            }
        }

        Ok(summary)
    }

    /// 获取每日趋势
    pub fn get_daily_trends(
        state: &AppState,
        _start_date: Option<i64>,
        _end_date: Option<i64>,
    ) -> Result<Vec<DailyStats>, AppError> {
        // 简化实现：返回最近7天的数据
        let mut trends = Vec::new();

        for i in 0..7 {
            let date = (Utc::now() - chrono::Duration::days(i))
                .format("%Y-%m-%d")
                .to_string();
            let key = format!("usage_daily_{}", date);

            if let Some(s) = state.db.get_setting(&key)? {
                if let Ok(stats) = serde_json::from_str::<DailyStats>(&s) {
                    trends.push(stats);
                }
            }
        }

        Ok(trends)
    }

    /// 获取供应商统计
    pub fn get_provider_stats(state: &AppState) -> Result<Vec<ProviderStats>, AppError> {
        let mut stats = Vec::new();

        // 从设置中获取供应商统计
        if let Some(s) = state.db.get_setting("usage_providers")? {
            if let Ok(provider_stats) = serde_json::from_str::<Vec<ProviderStats>>(&s) {
                stats = provider_stats;
            }
        }

        Ok(stats)
    }

    /// 获取模型统计
    pub fn get_model_stats(state: &AppState) -> Result<Vec<ModelStats>, AppError> {
        let mut stats = Vec::new();

        // 从设置中获取模型统计
        if let Some(s) = state.db.get_setting("usage_models")? {
            if let Ok(model_stats) = serde_json::from_str::<Vec<ModelStats>>(&s) {
                stats = model_stats;
            }
        }

        Ok(stats)
    }

    /// 检查限额
    pub fn check_limits(
        state: &AppState,
        provider_id: &str,
        _app_type: &str,
    ) -> Result<LimitStatus, AppError> {
        let daily_limit_key = format!("limit_daily_{}", provider_id);
        let monthly_limit_key = format!("limit_monthly_{}", provider_id);

        let daily_limit: Option<f64> = state.db.get_setting(&daily_limit_key)?
            .and_then(|s| s.parse().ok());

        let monthly_limit: Option<f64> = state.db.get_setting(&monthly_limit_key)?
            .and_then(|s| s.parse().ok());

        // 获取今日使用量
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let daily_key = format!("usage_daily_{}", today);

        let daily_used = if let Some(s) = state.db.get_setting(&daily_key)? {
            serde_json::from_str::<DailyStats>(&s)
                .map(|s| s.cost)
                .unwrap_or(0.0)
        } else {
            0.0
        };

        // 简化：月使用量假设等于日使用量
        let monthly_used = daily_used;

        let is_exceeded = daily_limit.map(|l| daily_used >= l).unwrap_or(false)
            || monthly_limit.map(|l| monthly_used >= l).unwrap_or(false);

        Ok(LimitStatus {
            provider_id: provider_id.to_string(),
            daily_limit,
            daily_used,
            monthly_limit,
            monthly_used,
            is_exceeded,
        })
    }

    /// 设置日限额
    pub fn set_daily_limit(
        state: &AppState,
        provider_id: &str,
        limit: f64,
    ) -> Result<(), AppError> {
        let key = format!("limit_daily_{}", provider_id);
        state.db.set_setting(&key, &limit.to_string())
    }

    /// 设置月限额
    pub fn set_monthly_limit(
        state: &AppState,
        provider_id: &str,
        limit: f64,
    ) -> Result<(), AppError> {
        let key = format!("limit_monthly_{}", provider_id);
        state.db.set_setting(&key, &limit.to_string())
    }

    /// 清除使用记录
    pub fn clear_usage(state: &AppState) -> Result<(), AppError> {
        // 简化实现：清除汇总数据
        state.db.delete_setting("usage_summary")?;
        state.db.delete_setting("usage_providers")?;
        state.db.delete_setting("usage_models")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_summary() {
        let summary = UsageSummary {
            total_requests: 100,
            total_tokens: 10000,
            input_tokens: 6000,
            output_tokens: 4000,
            total_cost: 1.5,
            period_start: None,
            period_end: None,
        };

        assert_eq!(summary.total_requests, 100);
        assert_eq!(summary.total_tokens, 10000);
    }
}