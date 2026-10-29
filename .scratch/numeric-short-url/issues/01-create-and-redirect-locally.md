# 01: 최소 관통: 생성하고 이동하기 (로컬)

**What to build:** 로컬에서 익명 방문자가 Dioxus 화면에 원본 URL을 입력하고 생성 버튼을 누르면 단축 주소가 표시되고, 그 단축 주소를 열면 서버의 302 리디렉션으로 원본 URL에 도착한다. workspace(프론트엔드·백엔드·공유 타입), Docker Postgres, 링크 테이블과 자동 마이그레이션, API 통합 테스트 환경(seam ①)까지 이 티켓에서 처음 세운다. 검증·404·불변성 같은 규칙은 이후 티켓에서 다룬다.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [x] Cargo workspace에 프론트엔드, 백엔드, 공유 타입 crate가 있고 전체가 빌드된다
- [x] Docker로 로컬 Postgres를 띄우는 방법이 정해져 있다 (배포 DB와 같은 메이저 버전)
- [x] 백엔드가 시작될 때 마이그레이션이 자동 실행되어, 번호(`GENERATED ALWAYS AS IDENTITY` 기본키, 1부터)와 원본 URL을 가진 링크 테이블이 생긴다
- [x] `POST /api/links`에 `{ "url": ... }`를 보내면 `201 { "number": ... }`를 응답한다
- [x] `GET /{번호}`가 `302`와 `Location: <원본 URL>`을 응답한다
- [x] 프론트엔드가 상대경로로 API를 호출하고, 현재 origin과 번호로 조립한 단축 주소를 화면에 표시한다
- [x] seam ① 통합 테스트(실제 Postgres에 연결한 Axum 앱에 HTTP 요청)가 생성 → 리디렉션 흐름을 검증하고 통과한다
- [x] 로컬 개발 서버로 화면에서 생성한 단축 주소를 브라우저로 열면 원본 URL로 이동한다
