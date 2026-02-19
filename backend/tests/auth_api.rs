mod common;

use axum::http::{header, StatusCode};
use common::create_auth_test_server as create_test_server;
use serde_json::json;

// Why: Session cookies must have HttpOnly to prevent XSS-based session theft.
#[tokio::test]
async fn test_register_creates_user_and_returns_session_cookie() {
    let server = create_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    response.assert_status(StatusCode::CREATED);

    let body: serde_json::Value = response.json();
    assert_eq!(body["user"]["email"], "test@example.com");
    assert_eq!(body["user"]["name"], "Test User");
    assert!(body["user"]["id"].is_string());

    // Check Set-Cookie header
    let cookies = response.headers().get("set-cookie");
    assert!(cookies.is_some(), "Should set session cookie");
    let cookie_str = cookies.unwrap().to_str().unwrap();
    assert!(cookie_str.contains("gantry_session="));
    assert!(cookie_str.contains("HttpOnly"));
}

#[tokio::test]
async fn test_register_validates_email() {
    let server = create_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "not-an-email",
            "name": "Test User",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

// Why: Short passwords are trivially brute-forced; enforcing minimum length
// raises the cost of offline attacks on leaked hashes.
#[tokio::test]
async fn test_register_validates_password_length() {
    let server = create_test_server().await;

    let response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "short"  // Less than 8 characters
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

// Why: Returning a specific "email already exists" error enables user enumeration
// attacks — attackers can probe the registration endpoint to discover valid accounts.
#[tokio::test]
async fn test_register_duplicate_email_returns_generic_error() {
    let server = create_test_server().await;

    let body = json!({
        "email": "test@example.com",
        "name": "Test User",
        "password": "Tr0ub4dor&3-correct-horse"
    });

    // First registration should succeed
    server.post("/api/auth/register").json(&body).await;

    // Second registration should fail with a generic message (not 409 Conflict)
    // to prevent user enumeration via registration
    let response = server.post("/api/auth/register").json(&body).await;

    response.assert_status(StatusCode::BAD_REQUEST);

    let error_body: serde_json::Value = response.json();
    let error_msg = error_body["error"]["message"].as_str().unwrap_or("");
    assert!(
        !error_msg.contains("email"),
        "error message should not reveal email exists: {error_msg}"
    );
    assert!(
        !error_msg.contains("already exists"),
        "error message should not reveal resource exists: {error_msg}"
    );
}

#[tokio::test]
async fn test_login_with_valid_credentials() {
    let server = create_test_server().await;

    // Register first
    server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    // Login
    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "email": "test@example.com",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["user"]["email"], "test@example.com");

    // Check Set-Cookie header
    let cookies = response.headers().get("set-cookie");
    assert!(cookies.is_some(), "Should set session cookie");
}

// Why: Invalid credentials must return UNAUTHORIZED (not a specific "wrong password"
// message) to prevent credential-stuffing bots from distinguishing valid accounts.
#[tokio::test]
async fn test_login_with_wrong_password() {
    let server = create_test_server().await;

    // Register first
    server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    // Login with wrong password
    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "email": "test@example.com",
            "password": "wrong_password"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

// Why: Must return the same status code as wrong-password to prevent user enumeration
// via response differences (no distinct error for unknown email vs wrong password).
#[tokio::test]
async fn test_login_with_nonexistent_email() {
    let server = create_test_server().await;

    let response = server
        .post("/api/auth/login")
        .json(&json!({
            "email": "nonexistent@example.com",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

// Why: Unauthenticated requests to user-specific endpoints must be rejected to enforce
// the authentication boundary and prevent information leakage.
#[tokio::test]
async fn test_me_without_auth() {
    let server = create_test_server().await;

    let response = server.get("/api/auth/me").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_with_auth() {
    let server = create_test_server().await;

    // Register and get session cookie
    let register_response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    let cookies = register_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Extract just the cookie value
    let cookie_value = cookies.split(';').next().unwrap();

    // Call /me with cookie
    let response = server
        .get("/api/auth/me")
        .add_header(header::COOKIE, cookie_value)
        .await;

    response.assert_status(StatusCode::OK);

    let body: serde_json::Value = response.json();
    assert_eq!(body["email"], "test@example.com");
    assert_eq!(body["name"], "Test User");
}

// Why: Logout must invalidate the server-side session AND set Max-Age=0 on the cookie.
// Without both, a stolen cookie could remain valid after the user logs out.
#[tokio::test]
async fn test_logout_clears_session() {
    let server = create_test_server().await;

    // Register and get session cookie
    let register_response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    let cookies = register_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let cookie_value = cookies.split(';').next().unwrap();

    // Logout
    let logout_response = server
        .post("/api/auth/logout")
        .add_header(header::COOKIE, cookie_value)
        .await;

    logout_response.assert_status(StatusCode::NO_CONTENT);

    // Check that the cookie is cleared
    let clear_cookie = logout_response.headers().get("set-cookie");
    assert!(clear_cookie.is_some());
    let clear_cookie_str = clear_cookie.unwrap().to_str().unwrap();
    assert!(clear_cookie_str.contains("Max-Age=0"));

    // Try to access /me with the old cookie - should fail
    let me_response = server
        .get("/api/auth/me")
        .add_header(header::COOKIE, cookie_value)
        .await;

    me_response.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_logout_without_auth() {
    let server = create_test_server().await;

    let response = server.post("/api/auth/logout").await;

    response.assert_status(StatusCode::UNAUTHORIZED);
}

// Why: When a user logs in again, old sessions must be invalidated to limit the
// window of exposure if a session token was compromised (session fixation prevention).
#[tokio::test]
async fn test_login_invalidates_previous_sessions() {
    let server = create_test_server().await;

    // Register user
    let register_response = server
        .post("/api/auth/register")
        .json(&json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    let old_cookie = register_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();

    // Login (creates new session, should invalidate old one)
    let _login_response = server
        .post("/api/auth/login")
        .json(&json!({
            "email": "test@example.com",
            "password": "Tr0ub4dor&3-correct-horse"
        }))
        .await;

    // Old session should now be invalid
    let me_response = server
        .get("/api/auth/me")
        .add_header(header::COOKIE, &*old_cookie)
        .await;

    me_response.assert_status(StatusCode::UNAUTHORIZED);
}

// Why: Rate limiting on login prevents brute-force and credential-stuffing attacks
// by throttling the number of attempts per time window.
#[tokio::test]
async fn test_login_rate_limit_returns_429_after_burst() {
    let server = create_test_server().await;

    let body = json!({
        "email": "nobody@example.com",
        "password": "doesnotmatter123"
    });

    // Exhaust burst_size (5) for login
    for _ in 0..5 {
        server.post("/api/auth/login").json(&body).await;
    }

    // 6th request should be rate-limited
    let response = server.post("/api/auth/login").json(&body).await;
    response.assert_status(StatusCode::TOO_MANY_REQUESTS);
}

// Why: Rate limiting on registration prevents automated mass account creation
// and abuse of the signup endpoint as an email oracle.
#[tokio::test]
async fn test_register_rate_limit_returns_429_after_burst() {
    let server = create_test_server().await;

    // Exhaust burst_size (3) for register — use different emails so the first ones succeed
    for i in 0..3 {
        let body = json!({
            "email": format!("user{}@example.com", i),
            "name": "Test User",
            "password": "Tr0ub4dor&3-correct-horse"
        });
        server.post("/api/auth/register").json(&body).await;
    }

    // 4th request should be rate-limited
    let body = json!({
        "email": "user3@example.com",
        "name": "Test User",
        "password": "Tr0ub4dor&3-correct-horse"
    });
    let response = server.post("/api/auth/register").json(&body).await;
    response.assert_status(StatusCode::TOO_MANY_REQUESTS);
}
