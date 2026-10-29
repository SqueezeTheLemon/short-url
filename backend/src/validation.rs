use url::{Origin, Url};

const MAX_LEN: usize = 2048;

/// 원본 URL을 검사하고, 통과하면 저장할 정규화된 문자열을 돌려줘요 (ADR-0005).
/// 실패하면 화면에 그대로 보여줄 한국어 메시지를 돌려줘요.
pub fn normalize_original_url(input: &str, public_origin: Option<&Origin>) -> Result<String, &'static str> {
    let input = input.trim();
    if input.is_empty() {
        return Err("원본 URL을 입력해 주세요.");
    }
    if input.chars().count() > MAX_LEN {
        return Err("URL이 너무 길어요. 2048자 이하만 단축할 수 있어요.");
    }

    let url = Url::parse(input).map_err(|_| "올바른 URL 형식이 아니에요. https://로 시작하는 전체 주소를 넣어 주세요.")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("http 또는 https 주소만 단축할 수 있어요.");
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err("호스트(도메인)가 없는 주소예요.");
    }
    if public_origin.is_some_and(|origin| url.origin() == *origin) {
        return Err("이 서비스의 단축 주소는 다시 단축할 수 없어요.");
    }

    // 정규화하면서 퍼센트 인코딩 등으로 길어질 수 있어 한 번 더 확인해요.
    let normalized = String::from(url);
    if normalized.len() > MAX_LEN {
        return Err("URL이 너무 길어요. 2048자 이하만 단축할 수 있어요.");
    }
    Ok(normalized)
}
