use dioxus::prelude::*;
use gloo_net::http::Request;
use shared::{CreateLinkRequest, CreateLinkResponse};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut original_url = use_signal(String::new);
    let mut short_url = use_signal(|| None::<String>);
    let mut error = use_signal(|| None::<String>);

    let create = move |_| async move {
        error.set(None);
        match create_link(original_url()).await {
            Ok(number) => short_url.set(Some(format!("{}/{number}", current_origin()))),
            Err(message) => {
                short_url.set(None);
                error.set(Some(message));
            }
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
            button { onclick: create, "생성" }
            if let Some(url) = short_url() {
                p { a { href: "{url}", "{url}" } }
            }
            if let Some(message) = error() {
                p { "{message}" }
            }
        }
    }
}

async fn create_link(url: String) -> Result<i64, String> {
    let res = Request::post("/api/links")
        .json(&CreateLinkRequest { url })
        .map_err(|err| err.to_string())?
        .send()
        .await
        .map_err(|_| "서버에 연결하지 못했어요".to_string())?;

    if res.status() != 201 {
        return Err(format!("생성에 실패했어요 (HTTP {})", res.status()));
    }
    let created: CreateLinkResponse = res.json().await.map_err(|err| err.to_string())?;
    Ok(created.number)
}

/// 단축 주소 = 지금 접속한 공개 도메인 + `/번호`
fn current_origin() -> String {
    web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .unwrap_or_default()
}
