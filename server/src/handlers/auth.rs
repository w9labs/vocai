use axum::{
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use tracing;

#[derive(Deserialize)]
pub struct OAuthCallback {
    pub code: String,
    pub state: Option<String>,
}

pub async fn callback(
    axum::extract::Query(params): axum::extract::Query<OAuthCallback>,
) -> impl IntoResponse {
    tracing::info!("OAuth callback received with code: {}", params.code);

    let issuer_url = std::env::var("ISSUER_URL").unwrap_or_else(|_| "https://db.w9.nu".to_string());
    let token_url = format!("{}/oauth/token", issuer_url);
    
    let client = reqwest::Client::new();
    
    let response = client
        .post(&token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &params.code),
            ("redirect_uri", &format!("{}/auth/callback", vocai_base_url())),
        ])
        .basic_auth(
            std::env::var("OAUTH_CLIENT_ID").unwrap_or_else(|_| "vocai".to_string()),
            Some(std::env::var("OAUTH_CLIENT_SECRET").unwrap_or_else(|_| "secret".to_string())),
        )
        .send()
        .await;

    match response {
        Ok(resp) => {
            if let Ok(token_data) = resp.json::<serde_json::Value>().await {
                tracing::info!("Token received: {}", token_data);
                // Try multiple field names (OIDC standard vs w9-db custom)
                let user_id = token_data.get("sub")
                    .or_else(|| token_data.get("user_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                if let Ok(uid) = uuid::Uuid::parse_str(user_id) {
                    tracing::info!("Auth successful, user_id={}", uid);
                    crate::session::create_session_cookie(uid);
                    return (StatusCode::FOUND, axum::response::Redirect::to("/dashboard")).into_response();
                } else {
                    tracing::error!("Invalid user_id in token response: '{}'", user_id);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed").into_response()
                }
            } else {
                tracing::error!("Failed to parse token response");
                (StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed").into_response()
            }
        }
        Err(e) => {
            tracing::error!("Token exchange failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed").into_response()
        }
    }
}

pub async fn login() -> impl IntoResponse {
    let issuer_url = std::env::var("ISSUER_URL").unwrap_or_else(|_| "https://db.w9.nu".to_string());
    let oauth_url = format!(
        "{}/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid+profile+email",
        issuer_url,
        std::env::var("OAUTH_CLIENT_ID").unwrap_or_else(|_| "vocai".to_string()),
        format!("{}/auth/callback", vocai_base_url()),
    );
    
    tracing::info!("Redirecting to OAuth: {}", oauth_url);
    (StatusCode::FOUND, axum::response::Redirect::to(&oauth_url)).into_response()
}

pub async fn logout() -> impl IntoResponse {
    crate::session::clear_session_cookie()
}

fn vocai_base_url() -> String {
    std::env::var("VOCAI_BASE_URL").unwrap_or_else(|_| "https://vocai.top".to_string())
}
