//! Seam ①: 실제 Postgres에 연결한 앱에 HTTP 요청을 보내고, 응답만 확인해요.
//! 실행 전에 `docker compose up -d` 와 `.env`(DATABASE_URL)가 필요해요.

use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
    response::Response,
};
use backend::AppConfig;
use http_body_util::BodyExt;
use shared::{CreateLinkRequest, CreateLinkResponse, ErrorResponse};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::task::JoinSet;
use tower::ServiceExt;

const PUBLIC_ORIGIN: &str = "https://short.example";

async fn test_pool() -> PgPool {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL 환경변수가 필요해요");
    let pool = PgPool::connect(&database_url).await.unwrap();
    backend::migrate(&pool).await.unwrap();
    pool
}

fn app_with(pool: PgPool) -> Router {
    backend::app(pool, AppConfig { public_origin: Some(PUBLIC_ORIGIN.into()), static_dir: None })
}

async fn test_app() -> Router {
    app_with(test_pool().await)
}

/// 테스트를 여러 번 돌려도 URL이 겹치지 않게 붙이는 값이에요.
fn unique() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
}

async fn post_link(app: &Router, url: &str) -> Response {
    let body = serde_json::to_vec(&CreateLinkRequest { url: url.into() }).unwrap();
    app.clone()
        .oneshot(
            Request::post("/api/links")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn create_link(app: &Router, url: &str) -> i64 {
    let res = post_link(app, url).await;
    assert_eq!(res.status(), StatusCode::CREATED, "생성 실패: {url}");
    let created: CreateLinkResponse = serde_json::from_slice(&body_bytes(res).await).unwrap();
    created.number
}

async fn get(app: &Router, path: &str) -> Response {
    send(app, Method::GET, path).await
}

async fn send(app: &Router, method: Method, path: &str) -> Response {
    app.clone()
        .oneshot(Request::builder().method(method).uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn body_bytes(res: Response) -> Vec<u8> {
    res.into_body().collect().await.unwrap().to_bytes().to_vec()
}

async fn assert_redirects_to(app: &Router, number: i64, expected: &str) {
    let res = get(app, &format!("/{number}")).await;
    assert_eq!(res.status(), StatusCode::FOUND);
    assert_eq!(res.headers()[header::LOCATION], expected);
}

async fn assert_rejected(app: &Router, url: &str) {
    let res = post_link(app, url).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "거부돼야 해요: {url}");
    let body: ErrorResponse = serde_json::from_slice(&body_bytes(res).await).unwrap();
    assert!(!body.error.is_empty());
}

async fn assert_not_found_page(app: &Router, path: &str) {
    let res = get(app, path).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND, "404여야 해요: {path}");
    assert!(res.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/html"));
    let html = String::from_utf8(body_bytes(res).await).unwrap();
    assert!(html.contains("href=\"/\""), "홈으로 가는 링크가 있어야 해요");
}

// ── 01: 생성과 리디렉션 ──────────────────────────────────────────

#[tokio::test]
async fn created_link_redirects_to_original_url() {
    let app = test_app().await;
    let original_url = "https://example.com/docs/page?lang=ko#intro";

    let number = create_link(&app, original_url).await;

    assert_redirects_to(&app, number, original_url).await;
}

// ── 02: 헬스체크 ────────────────────────────────────────────────

#[tokio::test]
async fn health_is_ok_when_db_is_reachable() {
    let app = test_app().await;
    assert_eq!(get(&app, "/health").await.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_is_unavailable_when_db_is_unreachable() {
    // 아무도 듣지 않는 포트(1)를 가리키는 풀이라 쿼리가 반드시 실패해요.
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .connect_lazy("postgres://postgres:postgres@127.0.0.1:1/none")
        .unwrap();
    let app = app_with(pool);

    assert_eq!(get(&app, "/health").await.status(), StatusCode::SERVICE_UNAVAILABLE);
}

// ── 04: 입력 검증 ───────────────────────────────────────────────

#[tokio::test]
async fn rejects_non_http_schemes() {
    let app = test_app().await;
    assert_rejected(&app, "javascript:alert(1)").await;
    assert_rejected(&app, "ftp://example.com/file").await;
    assert_rejected(&app, "mailto:someone@example.com").await;
}

#[tokio::test]
async fn rejects_unparseable_and_hostless_urls() {
    let app = test_app().await;
    assert_rejected(&app, "").await;
    assert_rejected(&app, "   ").await;
    assert_rejected(&app, "example.com").await;
    assert_rejected(&app, "not a url").await;
    assert_rejected(&app, "https://").await;
}

#[tokio::test]
async fn rejects_urls_longer_than_2048_chars() {
    let app = test_app().await;
    let prefix = "https://example.com/";

    let just_fits = format!("{prefix}{}", "a".repeat(2048 - prefix.len()));
    let too_long = format!("{just_fits}a");

    assert_eq!(post_link(&app, &just_fits).await.status(), StatusCode::CREATED);
    assert_rejected(&app, &too_long).await;
}

#[tokio::test]
async fn rejects_own_short_urls() {
    let app = test_app().await;
    assert_rejected(&app, &format!("{PUBLIC_ORIGIN}/1")).await;
    assert_rejected(&app, &format!("{PUBLIC_ORIGIN}/")).await;
}

#[tokio::test]
async fn trims_and_stores_normalized_url() {
    let app = test_app().await;

    let number = create_link(&app, "  HTTPS://Example.COM  ").await;

    assert_redirects_to(&app, number, "https://example.com/").await;
}

#[tokio::test]
async fn keeps_path_query_and_fragment() {
    let app = test_app().await;
    let original_url = "https://example.com/a/b/c?x=1&y=two%20words#section-2";

    let number = create_link(&app, original_url).await;

    assert_redirects_to(&app, number, original_url).await;
}

#[tokio::test]
async fn percent_encodes_non_ascii_paths() {
    let app = test_app().await;

    let number = create_link(&app, "https://ko.wikipedia.org/wiki/러스트").await;

    assert_redirects_to(&app, number, "https://ko.wikipedia.org/wiki/%EB%9F%AC%EC%8A%A4%ED%8A%B8").await;
}

// ── 05: 404 ─────────────────────────────────────────────────────

#[tokio::test]
async fn unknown_number_shows_not_found_page() {
    let app = test_app().await;
    assert_not_found_page(&app, &format!("/{}", i64::MAX)).await;
}

#[tokio::test]
async fn malformed_numbers_show_not_found_page() {
    let app = test_app().await;
    let number = create_link(&app, "https://example.com/leading-zero").await;

    // 실제로 있는 번호라도 선행 0을 붙이면 다른 표기라 404예요.
    assert_not_found_page(&app, &format!("/0{number}")).await;
    assert_not_found_page(&app, "/0").await;
    assert_not_found_page(&app, "/00").await;
    assert_not_found_page(&app, "/9223372036854775808").await; // i64::MAX + 1
    assert_not_found_page(&app, "/99999999999999999999999999").await;
}

// ── 06: 링크 무결성 ─────────────────────────────────────────────

#[tokio::test]
async fn links_cannot_be_modified_or_deleted() {
    let app = test_app().await;
    let original_url = format!("https://example.com/immutable/{}", unique());
    let number = create_link(&app, &original_url).await;

    for method in [Method::PUT, Method::PATCH, Method::DELETE, Method::POST] {
        let res = send(&app, method.clone(), &format!("/{number}")).await;
        assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED, "{method} /{number}");
    }
    for method in [Method::PUT, Method::PATCH, Method::DELETE] {
        let res = send(&app, method.clone(), "/api/links").await;
        assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED, "{method} /api/links");
    }

    assert_redirects_to(&app, number, &original_url).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_creates_get_unique_numbers() {
    let app = test_app().await;
    let run = unique();

    let mut tasks = JoinSet::new();
    for i in 0..50 {
        let app = app.clone();
        let url = format!("https://example.com/concurrent/{run}/{i}");
        tasks.spawn(async move { (create_link(&app, &url).await, url) });
    }
    let created = tasks.join_all().await;

    let numbers: HashSet<i64> = created.iter().map(|(number, _)| *number).collect();
    assert_eq!(numbers.len(), 50, "번호가 겹치면 안 돼요");
    for (number, url) in &created {
        assert_redirects_to(&app, *number, url).await;
    }
}

#[tokio::test]
async fn links_survive_app_restart() {
    let original_url = format!("https://example.com/restart/{}", unique());
    let number = create_link(&test_app().await, &original_url).await;

    // 풀까지 새로 만든 "다시 띄운 서버"로 확인해요.
    let restarted = test_app().await;

    assert_redirects_to(&restarted, number, &original_url).await;
}

#[tokio::test]
async fn same_url_gets_a_new_number_each_time() {
    let app = test_app().await;
    let original_url = format!("https://example.com/same/{}", unique());

    let first = create_link(&app, &original_url).await;
    let second = create_link(&app, &original_url).await;

    assert_ne!(first, second);
    assert_redirects_to(&app, first, &original_url).await;
    assert_redirects_to(&app, second, &original_url).await;
}
