# Cloudflare Worker를 유일한 공개 접점으로 두고 Railway로 프록시한다

제출한 공개 주소의 `/번호`가 실제로 리디렉션돼야 하므로, 공개 도메인은 Cloudflare 하나로 둔다. Worker가 Dioxus web 정적 파일을 서빙하고 `/api/*`와 `/번호` 요청은 Railway의 Axum 백엔드로 그대로 전달한다(3xx는 따라가지 않고 브라우저에 넘긴다). 프론트엔드는 상대경로로만 API를 부르므로 CORS와 브라우저 쪽 백엔드 주소 설정이 필요 없다.

## Considered Options

- **Dioxus fullstack(서버 함수)**: 프론트와 서버가 한 프로젝트에 섞여, Cloudflare(화면)와 Railway(서버)로 나눠 배포하는 과제 구조와 맞지 않아 기각.
- **화면은 Cloudflare, 단축 주소는 Railway 도메인**: 공개 주소가 둘로 갈리고 CORS가 필요해 기각.
