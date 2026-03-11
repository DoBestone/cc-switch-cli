//! Web 前端
//!
//! 提供简单的 HTML/CSS/JS 用户界面。

use axum::response::Html;

/// 主页面 HTML
pub async fn index() -> Html<String> {
    Html(include_str!("index.html").to_string())
}