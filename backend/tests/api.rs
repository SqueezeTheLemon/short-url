//! Seam ①: 실제 Postgres에 연결한 앱에 HTTP 요청을 보내고, 응답만 확인해요.
//! 실행 전에 `docker compose up -d` 와 `.env`(DATABASE_URL)가 필요해요.

use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use shared::{CreateLinkRequest, CreateLinkResponse};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;

async fn test_app() -> Router {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL 환경변수가 필요해요");
    let pool = PgPool::connect(&database_url).await.unwrap();
    backend::migrate(&pool).await.unwrap();
    backend::app(pool, None)
}

async fn create_link(app: &Router, url: &str) -> CreateLinkResponse {
    let body = serde_json::to_vec(&CreateLinkRequest { url: url.into() }).unwrap();
    let res = app
        .clone()
        .oneshot(
            Request::post("/api/links")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn created_link_redirects_to_original_url() {
    let app = test_app().await;
    let original_url = "https://example.com/docs/page?lang=ko#intro";

    let created = create_link(&app, original_url).await;
    let res = app
        .oneshot(Request::get(format!("/{}", created.number)).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::FOUND);
    assert_eq!(res.headers()[header::LOCATION], original_url);
}

#[tokio::test]
async fn health_is_ok_when_db_is_reachable() {
    let app = test_app().await;
    let res = app.oneshot(Request::get("/health").body(Body::empty()).unwrap()).await.unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_is_unavailable_when_db_is_unreachable() {
    // 아무도 듣지 않는 포트(1)를 가리키는 풀이라 쿼리가 반드시 실패해요.
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .connect_lazy("postgres://postgres:postgres@127.0.0.1:1/none")
        .unwrap();
    let app = backend::app(pool, None);
    let res = app.oneshot(Request::get("/health").body(Body::empty()).unwrap()).await.unwrap();

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}
