//! 프론트엔드와 백엔드가 함께 쓰는 API 타입이에요.

use serde::{Deserialize, Serialize};

/// `POST /api/links` 요청 본문
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLinkRequest {
    pub url: String,
}

/// `POST /api/links` 성공 응답 (201)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLinkResponse {
    pub number: i64,
}
