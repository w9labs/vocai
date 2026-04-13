use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect},
    extract::Query,
};
use cookie::Cookie;
use serde::Deserialize;
use tracing;

const W9_DB_URL: &str = "https://db.w9.nu";

#[derive(Deserialize)]
pub struct OAuthCallback {
    pub code: String,
}

pub async fn login() -> impl IntoResponse {
    let oauth_url = format!(
        "{}/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai",
        W9_DB_URL,
    );
    tracing::info!("Redirecting to OAuth: {}", oauth_url);
    Redirect::to(&oauth_url).into_response()
}

pub async fn callback(Query(q): Query<OAuthCallback>) -> impl IntoResponse {
    tracing::info!("OAuth callback received, code: {}", q.code);

    let client = reqwest::Client::new();
    let res = match client
        .post(format!("{}/oauth/token", W9_DB_URL))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &q.code),
            ("redirect_uri", "https://vocai.top/oauth/callback"),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Token exchange failed: {}", e);
            let oauth_url = format!("{}/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai", W9_DB_URL);
            return Redirect::to(&oauth_url).into_response();
        }
    };

    let json = match res.json::<serde_json::Value>().await {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Failed to parse token response: {}", e);
            let oauth_url = format!("{}/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai", W9_DB_URL);
            return Redirect::to(&oauth_url).into_response();
        }
    };

    let token = match json.get("access_token").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => {
            tracing::error!("No access_token in response: {}", json);
            let oauth_url = format!("{}/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai", W9_DB_URL);
            return Redirect::to(&oauth_url).into_response();
        }
    };

    tracing::info!("Auth successful, setting session cookie");

    let mut response = Redirect::to("/dashboard").into_response();
    let mut cookie = Cookie::new("vocai_session", token);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(cookie::SameSite::Lax);
    cookie.set_max_age(time::Duration::days(7));
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).unwrap(),
    );
    response
}

pub async fn logout() -> impl IntoResponse {
    let mut response = Redirect::to("/").into_response();
    let mut cookie = Cookie::new("vocai_session", "");
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(cookie::SameSite::Lax);
    cookie.set_max_age(time::Duration::seconds(0));
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).unwrap(),
    );
    response
}

pub fn get_session_token(headers: &axum::http::HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?;
    let cookie_str = cookie_header.to_str().ok()?;
    for cookie in cookie_str.split(';') {
        let c = cookie.trim();
        if c.starts_with("vocai_session=") {
            return Some(c.trim_start_matches("vocai_session=").to_string());
        }
    }
    None
}
