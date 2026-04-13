use cookie::Cookie;
use axum::http::HeaderValue;
use axum::response::Response;
use uuid::Uuid;

pub fn create_session_cookie(user_id: Uuid) -> Response {
    let mut cookie = Cookie::new("session_id", user_id.to_string());
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(cookie::SameSite::Lax);
    cookie.set_max_age(time::Duration::days(7));

    let mut response = Response::new(axum::body::Body::empty());
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).unwrap(),
    );
    response
}

pub fn get_user_from_session(headers: &axum::http::HeaderMap) -> Option<Uuid> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|cookie_header| {
            let cookie_str = cookie_header.to_str().ok()?;
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if cookie.starts_with("session_id=") {
                    let value = cookie.trim_start_matches("session_id=");
                    return Uuid::parse_str(value).ok();
                }
            }
            None
        })
}

pub fn clear_session_cookie() -> Response {
    let mut cookie = Cookie::new("session_id", "");
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(cookie::SameSite::Lax);
    cookie.set_max_age(time::Duration::seconds(0));

    let mut response = Response::new(axum::body::Body::empty());
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        HeaderValue::from_str(&cookie.to_string()).unwrap(),
    );
    response
}
