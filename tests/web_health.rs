use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use oli_demo_rust_app::web::{router, AppState};
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let res = router(AppState)
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"ok");
}
