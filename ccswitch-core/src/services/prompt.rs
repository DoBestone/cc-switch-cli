//! Prompt 服务模块
//!
//! 提供 Prompt 的业务逻辑，包括配置同步到各应用。

use indexmap::IndexMap;
use std::fs;

use crate::app_config::AppType;
use crate::config::write_text_file;
use crate::error::AppError;
use crate::prompt::{get_prompt_path, Prompt};
use crate::store::AppState;

/// Prompt 服务
pub struct PromptService;

impl PromptService {
    /// 列出指定应用的所有 Prompts
    pub fn list(state: &AppState, app: AppType) -> Result<IndexMap<String, Prompt>, AppError> {
        state.db.get_all_prompts(app.as_str())
    }

    /// 获取单个 Prompt
    pub fn get(state: &AppState, app: AppType, id: &str) -> Result<Option<Prompt>, AppError> {
        state.db.get_prompt(app.as_str(), id)
    }

    /// 添加 Prompt
    pub fn add(state: &AppState, app: AppType, prompt: Prompt) -> Result<(), AppError> {
        // 检查 ID 是否已存在
        if state.db.get_prompt(app.as_str(), &prompt.id)?.is_some() {
            return Err(AppError::InvalidInput(format!(
                "Prompt '{}' 已存在",
                prompt.id
            )));
        }

        state.db.save_prompt(app.as_str(), &prompt)?;

        // 如果启用，同步到应用
        if prompt.enabled {
            Self::sync_to_app(state, app)?;
        }

        Ok(())
    }

    /// 更新 Prompt
    pub fn update(state: &AppState, app: AppType, prompt: Prompt) -> Result<(), AppError> {
        // 检查是否存在
        if state.db.get_prompt(app.as_str(), &prompt.id)?.is_none() {
            return Err(AppError::InvalidInput(format!(
                "Prompt '{}' 不存在",
                prompt.id
            )));
        }

        state.db.save_prompt(app.as_str(), &prompt)?;

        // 同步到应用
        Self::sync_to_app(state, app)?;

        Ok(())
    }

    /// 删除 Prompt
    pub fn remove(state: &AppState, app: AppType, id: &str) -> Result<(), AppError> {
        // 检查是否存在
        let prompt = state.db.get_prompt(app.as_str(), id)?;
        if prompt.is_none() {
            return Err(AppError::InvalidInput(format!("Prompt '{}' 不存在", id)));
        }

        let was_enabled = prompt.unwrap().enabled;

        state.db.delete_prompt(app.as_str(), id)?;

        // 如果删除的是启用的 Prompt，需要同步
        if was_enabled {
            Self::sync_to_app(state, app)?;
        }

        Ok(())
    }

    /// 启用 Prompt（同时禁用其他 Prompt）
    pub fn enable(state: &AppState, app: AppType, id: &str) -> Result<(), AppError> {
        // 检查是否存在
        if state.db.get_prompt(app.as_str(), id)?.is_none() {
            return Err(AppError::InvalidInput(format!("Prompt '{}' 不存在", id)));
        }

        // 禁用所有其他 Prompt
        let prompts = state.db.get_all_prompts(app.as_str())?;
        for (prompt_id, _) in prompts {
            if prompt_id != id {
                state.db.update_prompt_enabled(app.as_str(), &prompt_id, false)?;
            }
        }

        // 启用指定的 Prompt
        state.db.update_prompt_enabled(app.as_str(), id, true)?;

        // 同步到应用
        Self::sync_to_app(state, app)?;

        Ok(())
    }

    /// 禁用 Prompt
    pub fn disable(state: &AppState, app: AppType, id: &str) -> Result<(), AppError> {
        // 检查是否存在
        if state.db.get_prompt(app.as_str(), id)?.is_none() {
            return Err(AppError::InvalidInput(format!("Prompt '{}' 不存在", id)));
        }

        state.db.update_prompt_enabled(app.as_str(), id, false)?;

        // 同步到应用（清空 Prompt 文件）
        Self::sync_to_app(state, app)?;

        Ok(())
    }

    /// 从应用导入 Prompt
    pub fn import_from_app(state: &AppState, app: AppType) -> Result<Option<String>, AppError> {
        let path = get_prompt_path(&app);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;

        if content.trim().is_empty() {
            return Ok(None);
        }

        // 生成 ID
        let id = format!("imported-{}", chrono::Utc::now().timestamp());
        let name = format!("从 {} 导入", app.display_name());

        let prompt = Prompt::new(&id, &name, content).with_enabled(true);

        // 禁用其他 Prompt
        let prompts = state.db.get_all_prompts(app.as_str())?;
        for (prompt_id, _) in prompts {
            state.db.update_prompt_enabled(app.as_str(), &prompt_id, false)?;
        }

        state.db.save_prompt(app.as_str(), &prompt)?;

        Ok(Some(id))
    }

    /// 同步 Prompt 到应用
    pub fn sync_to_app(state: &AppState, app: AppType) -> Result<(), AppError> {
        let path = get_prompt_path(&app);

        // 获取启用的 Prompt
        let enabled_prompt = state.db.get_enabled_prompt(app.as_str())?;

        match enabled_prompt {
            Some(prompt) => {
                // 写入 Prompt 内容
                write_text_file(&path, &prompt.content)?;
            }
            None => {
                // 如果没有启用的 Prompt，清空文件（如果存在）
                if path.exists() {
                    write_text_file(&path, "")?;
                }
            }
        }

        Ok(())
    }

    /// 同步所有应用的 Prompt
    pub fn sync_all(state: &AppState) -> Result<(), AppError> {
        for app in AppType::all() {
            Self::sync_to_app(state, *app)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_service_add_and_list() {
        let state = AppState::memory().unwrap();

        let prompt = Prompt::new("test-prompt", "Test Prompt", "# Test Content");
        PromptService::add(&state, AppType::Claude, prompt).unwrap();

        let prompts = PromptService::list(&state, AppType::Claude).unwrap();
        assert_eq!(prompts.len(), 1);
        assert!(prompts.contains_key("test-prompt"));
    }

    #[test]
    fn test_prompt_service_enable() {
        let state = AppState::memory().unwrap();

        let prompt1 = Prompt::new("prompt1", "Prompt 1", "Content 1");
        let prompt2 = Prompt::new("prompt2", "Prompt 2", "Content 2");

        PromptService::add(&state, AppType::Claude, prompt1).unwrap();
        PromptService::add(&state, AppType::Claude, prompt2).unwrap();

        PromptService::enable(&state, AppType::Claude, "prompt1").unwrap();

        let prompts = PromptService::list(&state, AppType::Claude).unwrap();
        assert!(prompts.get("prompt1").unwrap().enabled);
        assert!(!prompts.get("prompt2").unwrap().enabled);

        // 启用另一个应该禁用第一个
        PromptService::enable(&state, AppType::Claude, "prompt2").unwrap();

        let prompts = PromptService::list(&state, AppType::Claude).unwrap();
        assert!(!prompts.get("prompt1").unwrap().enabled);
        assert!(prompts.get("prompt2").unwrap().enabled);
    }
}
