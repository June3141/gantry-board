use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

/// CSRF protection middleware: requires `X-Requested-With` header on state-changing requests.
///
/// This prevents cross-origin form submissions because CORS blocks custom headers
/// from cross-origin requests unless explicitly allowed.
pub async fn csrf_check(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let needs_check = matches!(*req.method(), Method::POST | Method::PATCH | Method::DELETE);

    if needs_check && !req.headers().contains_key("x-requested-with") {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware;
    use axum::routing::{get, post};
    use axum::Router;
    use tower::ServiceExt;

    use super::csrf_check;

    fn test_app() -> Router {
        Router::new()
            .route("/test", post(|| async { "ok" }))
            .route("/test", get(|| async { "ok" }))
            .layer(middleware::from_fn(csrf_check))
    }

    #[tokio::test]
    async fn test_csrf_blocks_post_without_header() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_csrf_allows_post_with_header() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/test")
            .header("X-Requested-With", "XMLHttpRequest")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_csrf_allows_get_without_header() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
