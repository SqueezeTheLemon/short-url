//! Seam ②: 앱을 거치지 않고 SQL로 직접 바꾸려 해도 DB 트리거가 거부하는지 확인해요.

use sqlx::PgPool;

async fn test_pool() -> PgPool {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL 환경변수가 필요해요");
    let pool = PgPool::connect(&database_url).await.unwrap();
    backend::migrate(&pool).await.unwrap();
    pool
}

async fn insert_link(pool: &PgPool, url: &str) -> i64 {
    sqlx::query_scalar("INSERT INTO links (original_url) VALUES ($1) RETURNING number")
        .bind(url)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn stored_url(pool: &PgPool, number: i64) -> String {
    sqlx::query_scalar("SELECT original_url FROM links WHERE number = $1")
        .bind(number)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn db_rejects_update() {
    let pool = test_pool().await;
    let number = insert_link(&pool, "https://example.com/before").await;

    let result = sqlx::query("UPDATE links SET original_url = 'https://evil.example' WHERE number = $1")
        .bind(number)
        .execute(&pool)
        .await;

    assert!(result.is_err(), "UPDATE가 거부돼야 해요");
    assert_eq!(stored_url(&pool, number).await, "https://example.com/before");
}

#[tokio::test]
async fn db_rejects_delete() {
    let pool = test_pool().await;
    let number = insert_link(&pool, "https://example.com/keep-me").await;

    let result = sqlx::query("DELETE FROM links WHERE number = $1").bind(number).execute(&pool).await;

    assert!(result.is_err(), "DELETE가 거부돼야 해요");
    assert_eq!(stored_url(&pool, number).await, "https://example.com/keep-me");
}

#[tokio::test]
async fn db_rejects_truncate() {
    let pool = test_pool().await;
    let number = insert_link(&pool, "https://example.com/survives-truncate").await;

    let result = sqlx::query("TRUNCATE links").execute(&pool).await;

    assert!(result.is_err(), "TRUNCATE가 거부돼야 해요");
    assert_eq!(stored_url(&pool, number).await, "https://example.com/survives-truncate");
}
