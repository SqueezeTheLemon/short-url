use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use shared::{CreateLinkRequest, CreateLinkResponse};
use sqlx::PgPool;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    static_files: Option<ServeDir<ServeFile>>,
}

/// 서버 시작 시 마이그레이션을 적용해요. 이미 적용된 건 건너뛰어서 여러 번 불러도 안전해요.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(pool).await
}

/// `static_dir`를 주면 dx 빌드 산출물(화면)도 같이 서빙해요.
/// 배포에서는 Cloudflare Worker가 이 역할을 하고(ADR-0001), 로컬에서 한 주소로 확인할 때만 써요.
pub fn app(pool: PgPool, static_dir: Option<PathBuf>) -> Router {
    let static_files = static_dir
        .map(|dir| ServeDir::new(&dir).fallback(ServeFile::new(dir.join("index.html"))));

    Router::new()
        .route("/health", get(health))
        .route("/api/links", post(create_link))
        // `/{번호}`를 라우트로 두면 `/favicon.ico` 같은 경로까지 잡혀요.
        // 그래서 Worker처럼 "숫자로만 된 경로인가?"를 fallback에서 직접 판단해요.
        .fallback(fallback)
        .with_state(AppState { pool, static_files })
}

/// Render가 새 버전을 띄운 뒤 이 경로가 200을 줄 때만 트래픽을 넘겨요.
/// 서버만 살아있고 DB에 못 붙는 상태도 "비정상"으로 보이도록 DB까지 확인해요.
async fn health(State(state): State<AppState>) -> StatusCode {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => StatusCode::OK,
        Err(err) => {
            eprintln!("health check failed: {err}");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

async fn create_link(
    State(state): State<AppState>,
    Json(req): Json<CreateLinkRequest>,
) -> Result<(StatusCode, Json<CreateLinkResponse>), StatusCode> {
    let number: i64 =
        sqlx::query_scalar("INSERT INTO links (original_url) VALUES ($1) RETURNING number")
            .bind(&req.url)
            .fetch_one(&state.pool)
            .await
            .map_err(internal_error)?;

    Ok((StatusCode::CREATED, Json(CreateLinkResponse { number })))
}

async fn fallback(State(state): State<AppState>, req: Request) -> Response {
    if let Some(number) = parse_number(req.uri().path()) {
        return redirect(&state.pool, number).await;
    }
    match state.static_files {
        Some(files) => files.oneshot(req).await.into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// `/123` → `Some(123)`. 선행 0·범위 규칙은 티켓 05에서 다듬어요.
fn parse_number(path: &str) -> Option<i64> {
    let digits = path.strip_prefix('/')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

async fn redirect(pool: &PgPool, number: i64) -> Response {
    let found: Option<String> =
        match sqlx::query_scalar("SELECT original_url FROM links WHERE number = $1")
            .bind(number)
            .fetch_optional(pool)
            .await
        {
            Ok(found) => found,
            Err(err) => return internal_error(err).into_response(),
        };

    let Some(original_url) = found else {
        return StatusCode::NOT_FOUND.into_response();
    };
    // axum의 Redirect::to 는 303이라, 302(Found)를 직접 만들어요.
    match HeaderValue::try_from(original_url) {
        Ok(location) => (StatusCode::FOUND, [(header::LOCATION, location)]).into_response(),
        Err(err) => internal_error(err).into_response(),
    }
}

fn internal_error(err: impl std::fmt::Display) -> StatusCode {
    eprintln!("internal error: {err}");
    StatusCode::INTERNAL_SERVER_ERROR
}
