use sqlx::PgPool;

#[tokio::main]
async fn main() {
    // 로컬에선 .env 를 읽고, 배포 환경(Render)에선 진짜 환경변수를 그대로 써요.
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL 환경변수가 필요해요");
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let config = backend::AppConfig {
        // 배포에선 꼭 넣어야 자기 단축 주소를 다시 단축하는 걸 막을 수 있어요.
        public_origin: std::env::var("PUBLIC_ORIGIN").ok(),
        static_dir: std::env::var_os("STATIC_DIR").map(Into::into),
    };

    let pool = PgPool::connect(&database_url).await.expect("DB에 연결하지 못했어요");
    backend::migrate(&pool).await.expect("마이그레이션에 실패했어요");

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port.parse::<u16>().expect("PORT는 숫자여야 해요")))
        .await
        .expect("포트를 열지 못했어요");
    println!("listening on http://localhost:{port}");
    axum::serve(listener, backend::app(pool, config)).await.unwrap();
}
