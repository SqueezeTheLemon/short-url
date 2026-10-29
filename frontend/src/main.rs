use dioxus::prelude::*;
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use shared::{CreateLinkRequest, CreateLinkResponse, ErrorResponse};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

fn main() {
    dioxus::launch(App);
}

/// 숫자로만 된 경로(`/123`)는 Worker가 백엔드로 보내서 여기까지 오지 않아요.
/// 그 밖의 미지의 경로는 화면의 "페이지 없음"이 맡아요.
#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

#[component]
fn App() -> Element {
    rsx! { Router::<Route> {} }
}

#[derive(Clone, PartialEq)]
enum CopyStatus {
    Idle,
    Copied,
    Failed,
}

#[component]
fn Home() -> Element {
    let mut original_url = use_signal(String::new);
    let mut short_url = use_signal(|| None::<String>);
    let mut error = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let mut copy_status = use_signal(|| CopyStatus::Idle);

    let create = move |_| async move {
        // 버튼이 비활성화돼 있어도, 혹시 모를 중복 클릭은 여기서 한 번 더 막아요.
        if loading() {
            return;
        }
        loading.set(true);
        error.set(None);
        copy_status.set(CopyStatus::Idle);

        match create_link(original_url()).await {
            Ok(number) => short_url.set(Some(format!("{}/{number}", current_origin()))),
            Err(message) => {
                short_url.set(None);
                error.set(Some(message));
            }
        }
        loading.set(false);
    };

    let copy = move |_| async move {
        let Some(url) = short_url() else { return };
        match copy_to_clipboard(&url).await {
            Ok(()) => {
                copy_status.set(CopyStatus::Copied);
                TimeoutFuture::new(2_000).await;
                // 그 사이 새 주소를 만들었다면 이미 Idle이라 건드리지 않아요.
                if copy_status() == CopyStatus::Copied {
                    copy_status.set(CopyStatus::Idle);
                }
            }
            Err(()) => copy_status.set(CopyStatus::Failed),
        }
    };

    rsx! {
        main {
            h1 { "숫자형 단축 URL" }
            input {
                r#type: "url",
                placeholder: "https://example.com/very/long/url",
                value: "{original_url}",
                oninput: move |evt| original_url.set(evt.value()),
            }
            button { disabled: loading(), onclick: create,
                if loading() { "생성 중…" } else { "생성" }
            }
            if let Some(url) = short_url() {
                p {
                    a { href: "{url}", "{url}" }
                    " "
                    button { onclick: copy, "복사" }
                    match copy_status() {
                        CopyStatus::Idle => rsx! {},
                        CopyStatus::Copied => rsx! { span { " 복사됨!" } },
                        CopyStatus::Failed => rsx! { span { " 복사하지 못했어요. 주소를 직접 선택해 복사해 주세요." } },
                    }
                }
            }
            if let Some(message) = error() {
                p { role: "alert", "{message}" }
            }
        }
    }
}

#[component]
fn NotFound(segments: Vec<String>) -> Element {
    rsx! {
        main {
            h1 { "페이지를 찾을 수 없어요" }
            p { "주소가 잘못 입력되지 않았는지 확인해 주세요." }
            Link { to: Route::Home {}, "홈으로 돌아가기" }
        }
    }
}

async fn create_link(url: String) -> Result<i64, String> {
    let res = Request::post("/api/links")
        .json(&CreateLinkRequest { url })
        .map_err(|err| err.to_string())?
        .send()
        .await
        .map_err(|_| "서버에 연결하지 못했어요. 잠시 후 다시 시도해 주세요.".to_string())?;

    match res.status() {
        201 => {
            let created: CreateLinkResponse = res.json().await.map_err(|err| err.to_string())?;
            Ok(created.number)
        }
        // 검증 실패는 서버가 보낸 한국어 메시지를 그대로 보여줘요.
        400 => match res.json::<ErrorResponse>().await {
            Ok(body) => Err(body.error),
            Err(_) => Err("입력한 URL을 확인해 주세요.".into()),
        },
        status => Err(format!("생성에 실패했어요 (HTTP {status})")),
    }
}

/// 클립보드 API는 HTTPS나 localhost에서만 있어요. 없으면 Err로 알려줘요.
async fn copy_to_clipboard(text: &str) -> Result<(), ()> {
    let navigator = web_sys::window().ok_or(())?.navigator();
    // 비보안 컨텍스트에선 navigator.clipboard 자체가 undefined라, 부르기 전에 확인해요.
    let clipboard = js_sys::Reflect::get(&navigator, &"clipboard".into()).map_err(|_| ())?;
    if clipboard.is_undefined() {
        return Err(());
    }
    let clipboard: web_sys::Clipboard = clipboard.unchecked_into();
    JsFuture::from(clipboard.write_text(text)).await.map(|_| ()).map_err(|_| ())
}

/// 단축 주소 = 지금 접속한 공개 도메인 + `/번호`
fn current_origin() -> String {
    web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .unwrap_or_default()
}
