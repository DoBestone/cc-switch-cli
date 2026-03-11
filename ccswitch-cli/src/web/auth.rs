//! 认证模块
//!
//! 提供 JWT 认证功能。

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// JWT 密钥
static JWT_SECRET: Lazy<Vec<u8>> = Lazy::new(|| {
    // 生成随机密钥
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("cc-switch-secret-{}", timestamp).into_bytes()
});

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // 用户名
    pub exp: usize,  // 过期时间
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub message: Option<String>,
}

/// 认证状态
#[derive(Clone)]
pub struct AuthState {
    pub username: String,
    pub password: String,
}

/// 生成 JWT Token
pub fn generate_token(username: &str) -> Result<String, String> {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as usize
        + 24 * 60 * 60; // 24小时有效期

    let claims = Claims {
        sub: username.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&JWT_SECRET),
    )
    .map_err(|e| e.to_string())
}

/// 验证 JWT Token
pub fn verify_token(token: &str) -> Result<Claims, String> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(&JWT_SECRET),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| e.to_string())
}

/// 登录处理
pub async fn login(
    State(auth_state): State<AuthState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    // 验证用户名和密码
    if req.username == auth_state.username && req.password == auth_state.password {
        match generate_token(&req.username) {
            Ok(token) => (
                StatusCode::OK,
                Json(LoginResponse {
                    success: true,
                    token: Some(token),
                    message: Some("登录成功".to_string()),
                }),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LoginResponse {
                    success: false,
                    token: None,
                    message: Some(format!("生成Token失败: {}", e)),
                }),
            ),
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(LoginResponse {
                success: false,
                token: None,
                message: Some("用户名或密码错误".to_string()),
            }),
        )
    }
}

/// 认证中间件
pub async fn auth_middleware(
    State(_auth_state): State<AuthState>,
    request: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    // 登录接口不需要认证
    let path = request.uri().path();
    if path == "/api/login" || path == "/" || path.starts_with("/static") {
        return next.run(request).await;
    }

    // 获取 Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h: &axum::http::HeaderValue| h.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            match verify_token(token) {
                Ok(_claims) => {
                    // 认证成功，继续处理请求
                    next.run(request).await
                }
                Err(e) => {
                    // Token 无效
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({
                            "success": false,
                            "message": format!("认证失败: {}", e)
                        })),
                    ).into_response()
                }
            }
        }
        _ => {
            // 没有 Token 或格式错误
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "success": false,
                    "message": "未提供认证Token"
                })),
            ).into_response()
        }
    }
}