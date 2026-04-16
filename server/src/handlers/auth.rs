use axum::{
    extract::Query,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Redirect},
};
use cookie::Cookie;
use serde::Deserialize;
use tracing;
use uuid::Uuid;

const W9_DB_URL: &str = "https://db.w9.nu";

#[derive(Deserialize)]
pub struct OAuthCallback {
    pub code: String,
}

fn login_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Vocai Login</title>
  <style>
    :root {{ --bg: #050505; --panel: #111111; --text: #e5e5e5; --accent: #00ff41; --border: #e5e5e5; }}
    * {{ box-sizing: border-box; }}
    body {{ margin: 0; min-height: 100vh; display: grid; place-items: center; background: var(--bg); color: var(--text); font-family: monospace; padding: 24px; }}
    .card {{ width: min(100%, 520px); background: var(--panel); border: 4px solid var(--border); box-shadow: 8px 8px 0 #000; padding: 24px; text-align: center; }}
    img {{ width: min(100%, 320px); height: auto; display: block; margin: 0 auto 16px; }}
    h1 {{ margin: 0 0 12px; }}
    p {{ margin: 0 0 20px; line-height: 1.5; }}
    .btn {{ display: inline-block; padding: 14px 18px; background: var(--accent); color: #000; border: 3px solid var(--border); box-shadow: 4px 4px 0 #000; text-decoration: none; font-weight: 700; }}
    .btn:hover {{ transform: translate(2px, 2px); box-shadow: 2px 2px 0 #000; }}
  </style>
</head>
<body>
  <div class="card">
    <img src="/w9-logo/logo-landscape-transparent.svg" alt="W9 Labs">
    <h1>Vocai</h1>
    <p>Sign in with W9 DB to sync your vocabulary progress.</p>
    <a class="btn" href="https://db.w9.nu/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai" onclick="const w=window.open(this.href,'w9-vocai-login','width=520,height=720'); if (w) { w.focus(); return false; }">Login with W9 DB</a>
  </div>
</body>
</html>"#
        .to_string()
}

fn popup_close_html(target: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Vocai Login</title></head><body><script>(function(){{const target = {target:?}; if (window.opener && !window.opener.closed) {{ try {{ window.opener.location.href = target; window.opener.focus(); }} catch (_) {{}} window.close(); }} else {{ window.location.replace(target); }}}})();</script><p>Signing you in…</p></body></html>"#
    )
}

pub async fn login(headers: HeaderMap) -> impl IntoResponse {
    if get_session(&headers).is_some() {
        return Redirect::to("/dashboard").into_response();
    }

    Html(login_html()).into_response()
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
            return Redirect::to("https://db.w9.nu/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai").into_response();
        }
    };

    let json = match res.json::<serde_json::Value>().await {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Failed to parse token response: {}", e);
            return Redirect::to("https://db.w9.nu/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai").into_response();
        }
    };

    let token = match json.get("access_token").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => {
            tracing::error!("No access_token in response: {}", json);
            return Redirect::to("https://db.w9.nu/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai").into_response();
        }
    };

    // Get user info from w9-db /api/auth/me
    let user_info = match client
        .get(format!("{}/api/auth/me", W9_DB_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to get user info: {}", e);
            return Redirect::to("https://db.w9.nu/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai").into_response();
        }
    };

    let user_json = match user_info.json::<serde_json::Value>().await {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Failed to parse user info: {}", e);
            return Redirect::to("https://db.w9.nu/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai").into_response();
        }
    };

    let email = user_json
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown@vocai.top")
        .to_string();

    // Look up or create user by email
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let user_id = match upsert_user(&db_url, &email).await {
        Ok(uid) => uid,
        Err(e) => {
            tracing::error!("Failed to upsert user: {}", e);
            return Redirect::to("https://db.w9.nu/oauth/authorize?redirect_uri=https://vocai.top/oauth/callback&response_type=code&client_id=vocai").into_response();
        }
    };

    tracing::info!("User authenticated: {} (id={})", email, user_id);

    // Set session cookie
    let mut response = Html(popup_close_html("/dashboard")).into_response();
    let mut cookie = Cookie::new("vocai_session", format!("{}:{}", user_id, token));
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

async fn upsert_user(
    db_url: &str,
    email: &str,
) -> Result<Uuid, Box<dyn std::error::Error + Send + Sync>> {
    let config: tokio_postgres::Config = db_url.parse()?;
    let manager = bb8_postgres::PostgresConnectionManager::new(config, tokio_postgres::NoTls);
    let pool = bb8::Pool::builder().max_size(5).build(manager).await?;

    let client = pool.get().await?;

    // Try to find existing user by email
    let existing = client
        .query_opt("SELECT id FROM users WHERE email = $1", &[&email])
        .await?;

    if let Some(row) = existing {
        return Ok(row.get::<_, Uuid>("id"));
    }

    // Create new user
    let new_id = Uuid::new_v4();
    client
        .execute(
            "INSERT INTO users (id, email) VALUES ($1, $2)",
            &[&new_id, &email],
        )
        .await?;

    tracing::info!("Created new user: {} (id={})", email, new_id);
    Ok(new_id)
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

/// Parse "user_id:token" from cookie, return both
pub fn get_session(headers: &axum::http::HeaderMap) -> Option<(Uuid, String)> {
    let cookie_header = headers.get(header::COOKIE)?;
    let cookie_str = cookie_header.to_str().ok()?;
    tracing::debug!("Cookie header: {}", cookie_str);
    for c in cookie_str.split(';') {
        let c = c.trim();
        if c.starts_with("vocai_session=") {
            let val = c.trim_start_matches("vocai_session=");
            tracing::debug!("Session cookie value: {}", val);
            if let Some((uid_str, token)) = val.split_once(':') {
                tracing::debug!("Parsed uid: {}, token len: {}", uid_str, token.len());
                if let Ok(uid) = Uuid::parse_str(uid_str) {
                    tracing::info!("Session valid for user: {}", uid);
                    return Some((uid, token.to_string()));
                } else {
                    tracing::warn!("Invalid UUID in session cookie: {}", uid_str);
                }
            } else {
                tracing::warn!("No colon found in session cookie value");
            }
            return None;
        }
    }
    tracing::debug!("No vocai_session cookie found");
    None
}
