# 03: Cloudflare 공개 주소 (Worker 프록시)

**What to build:** Cloudflare의 HTTPS 공개 주소 하나에서 화면과 단축 주소가 모두 동작한다. Worker가 Dioxus web 정적 파일을 서빙하고, `/api/*`와 숫자로만 된 경로는 Railway 백엔드로 프록시하며 백엔드의 3xx 리디렉션을 따라가지 않고 브라우저에 그대로 넘긴다. 시크릿 창에서 생성한 단축 주소를 다른 브라우저로 열면 원본 URL로 이동한다. (ADR-0001, ADR-0004)

**Blocked by:** 02 (백엔드를 Railway에 배포)

**Status:** ready-for-agent

- [ ] Cloudflare Workers + Static Assets 프로젝트가 있고, 로컬 `dx` 빌드 산출물과 Worker를 `wrangler deploy`로 배포할 수 있다
- [ ] 숫자로만 된 경로와 `/api/*`는 SPA fallback보다 먼저 Worker가 처리해 Railway로 프록시한다
- [ ] 백엔드의 `302` 응답과 `Location` 헤더가 브라우저까지 그대로 전달된다
- [ ] 백엔드 주소는 Worker 환경변수로 설정되고, 브라우저 번들에는 어떤 비밀값도 없다
- [ ] Railway의 `PUBLIC_ORIGIN`이 Cloudflare 공개 주소로 설정되어 있다
- [ ] 공개 주소가 HTTPS로 열리고, 시크릿 창에서 생성 → 다른 브라우저에서 이동이 확인된다
