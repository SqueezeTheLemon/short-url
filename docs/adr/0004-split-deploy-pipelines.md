# 백엔드는 GitHub 연동 자동 배포, 프론트엔드는 로컬 빌드 후 wrangler로 배포한다

Railway는 저장소의 Dockerfile(multi-stage)로 push마다 백엔드를 빌드·배포한다. workspace에 crate가 여럿이라 자동 감지 빌드는 실행 대상을 잘못 고르기 쉬워 Dockerfile로 명시한다. Cloudflare 쪽 빌드 환경에는 Rust와 `dx`가 없으므로, Dioxus web은 로컬에서 `dx`로 빌드하고 산출물과 Worker를 `wrangler deploy`로 올린다. 두 배포는 서로 독립적으로 진행된다.
