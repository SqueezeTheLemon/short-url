use std::path::PathBuf;

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use shared::{CreateLinkRequest, CreateLinkResponse, ErrorResponse};
use sqlx::PgPool;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};
use url::{Origin, Url};

mod validation;

const NOT_FOUND_HTML: &str = include_str!("not_found.html");

/// 환경변수에서 읽어 앱에 넘기는 설정이에요.
#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    /// 사용자가 접속하는 공개 주소(예: Cloudflare Worker). 자기 자신을 단축하는 걸 막을 때 써요.
    pub public_origin: Option<String>,
    /// 주면 dx 빌드 산출물(화면)도 같이 서빙해요.
    /// 배포에서는 Cloudflare Worker가 이 역할을 하고(ADR-0001), 로컬에서 한 주소로 확인할 때만 써요.
    pub static_dir: Option<PathBuf>,
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    public_origin: Option<Origin>,
    static_files: Option<ServeDir<ServeFile>>,
}

/// 서버 시작 시 마이그레이션을 적용해요. 이미 적용된 건 건너뛰어서 여러 번 불러도 안전해요.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(pool).await
}

pub fn app(pool: PgPool, config: AppConfig) -> Router {
    let public_origin = config.public_origin.map(|origin| {
        Url::parse(&origin)
            .expect("PUBLIC_ORIGIN은 https://example.com 같은 전체 주소여야 해요")
            .origin()
    });
    let static_files = config
        .static_dir
        .map(|dir| ServeDir::new(&dir).fallback(ServeFile::new(dir.join("index.html"))));

    Router::new()
        .route("/health", get(health))
        // POST만 등록돼 있어서 PUT/PATCH/DELETE는 axum이 알아서 405로 응답해요.
        .route("/api/links", post(create_link))
        // `/{번호}`를 라우트로 두면 `/favicon.ico` 같은 경로까지 잡혀요.
        // 그래서 Worker처럼 "숫자로만 된 경로인가?"를 fallback에서 직접 판단해요.
        .fallback(fallback)
        .with_state(AppState { pool, public_origin, static_files })
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

async fn create_link(State(state): State<AppState>, Json(req): Json<CreateLinkRequest>) -> Response {
    let original_url = match validation::normalize_original_url(&req.url, state.public_origin.as_ref()) {
        Ok(url) => url,
        Err(message) => {
            return (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: message.into() })).into_response();
        }
    };

    let inserted = sqlx::query_scalar("INSERT INTO links (original_url) VALUES ($1) RETURNING number")
        .bind(&original_url)
        .fetch_one(&state.pool)
        .await;
    match inserted {
        Ok(number) => (StatusCode::CREATED, Json(CreateLinkResponse { number })).into_response(),
        Err(err) => internal_error(err).into_response(),
    }
}

async fn fallback(State(state): State<AppState>, req: Request) -> Response {
    let path = req.uri().path();
    // Worker와 같은 기준(`^/[0-9]+$`)이라, 여기 걸린 경로는 화면으로 넘기지 않고 항상 백엔드가 답해요.
    if is_numeric_path(path) {
        if !matches!(*req.method(), Method::GET | Method::HEAD) {
            // 링크는 수정·삭제할 수 없어요. 허용되는 메서드를 Allow 헤더로 알려줘요.
            return (StatusCode::METHOD_NOT_ALLOWED, [(header::ALLOW, "GET, HEAD")]).into_response();
        }
        return match parse_number(path) {
            Some(number) => redirect(&state.pool, number).await,
            None => not_found_page(),
        };
    }
    match state.static_files {
        Some(files) => files.oneshot(req).await.into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn is_numeric_path(path: &str) -> bool {
    path.strip_prefix('/')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// 한 번호는 표기가 하나뿐이에요: `/123`만 맞고 `/0123`, `/0`, i64를 넘는 수는 없는 번호로 봐요.
fn parse_number(path: &str) -> Option<i64> {
    let digits = path.strip_prefix('/')?;
    if digits.starts_with('0') {
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
        return not_found_page();
    };
    // axum의 Redirect::to 는 303이라, 302(Found)를 직접 만들어요.
    match HeaderValue::try_from(original_url) {
        Ok(location) => (StatusCode::FOUND, [(header::LOCATION, location)]).into_response(),
        Err(err) => internal_error(err).into_response(),
    }
}

fn not_found_page() -> Response {
    (StatusCode::NOT_FOUND, Html(NOT_FOUND_HTML)).into_response()
}

fn internal_error(err: impl std::fmt::Display) -> StatusCode {
    eprintln!("internal error: {err}");
    StatusCode::INTERNAL_SERVER_ERROR
}
