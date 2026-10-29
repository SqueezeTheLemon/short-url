# 숫자형 단축 URL 서비스

## 배포 URL

- 서비스 URL: https://short-url.squeezethelime.workers.dev
- 테스트 단축 URL: https://short-url.squeezethelime.workers.dev/14
- 테스트 원본 URL: https://velog.io/@squeezethelemon/posts

---

로그인 없이 긴 URL을 `/123` 같은 숫자로만 된 단축 주소로 바꿔 주는 서비스예요. 단축 주소에 접속하면 서버의 HTTP `302` 리디렉션으로 원본 URL에 도착하고, 한 번 만든 링크는 수정·삭제·만료·재사용되지 않아요.

| 영역 | 기술 |
|---|---|
| 화면 | Dioxus 0.7 (web, WASM) |
| 서버 | Rust, Axum 0.8, sqlx 0.9 |
| DB | PostgreSQL 17 |
| 배포 | Cloudflare Workers + Static Assets(공개 주소), Render(서버), Neon(DB) |

## 같은 원본 URL 처리 규칙

**같은 원본 URL을 다시 입력해도 항상 새 번호를 발급해요.** 기존 번호를 찾아 돌려주지 않아요.

- 기존 번호를 돌려주려면 "`https://a.com`과 `https://a.com/`은 같은 URL인가" 같은 비교 규칙과, 같은 URL이 동시에 들어왔을 때의 경쟁 상태를 따로 풀어야 해요. 과제가 두 방식 중 선택을 허용해서 단순한 쪽을 골랐어요. ([ADR-0002](docs/adr/0002-always-issue-new-number.md))

## 번호 발급과 링크 무결성

### 숫자 발급 방식

- 링크 테이블의 기본키가 `BIGINT GENERATED ALWAYS AS IDENTITY`(1부터)예요. 번호는 **DB 시퀀스만** 발급하고, 애플리케이션은 번호를 계산하지 않아요.
- 시퀀스는 트랜잭션이 실패해도 되돌리지 않아서 **번호에 빈 구간이 생길 수 있어요.** 대신 같은 번호가 두 번 나오거나 재사용되는 일은 없어요.
- 한 번호는 표기가 하나뿐이에요. `/123`만 유효하고 `/0123`, `/0`, i64 범위를 넘는 수는 존재 여부와 관계없이 `404`예요.

### 중복 방지 방법

- 번호가 기본키(PK)라서 DB가 유일성을 보장해요. 여러 요청이 동시에 들어와도 `INSERT ... RETURNING number` 한 문장으로 각자 다른 번호를 받기 때문에, 서로의 링크를 덮어쓸 수 없어요.
- 통합 테스트가 서로 다른 URL 50개를 동시에 생성해 번호가 모두 다르고 각 번호가 자기 URL로 이동하는지 확인해요.

### 생성 후 불변을 지키는 방법 (2중 방어)

1. **서버**: 링크를 바꾸는 라우트가 없어요. 숫자 경로에 `GET`/`HEAD`가 아닌 요청(`PUT`, `PATCH`, `DELETE`, `POST`)이 오면 `405 Method Not Allowed`로 거부해요.
2. **DB**: 트리거가 링크 테이블의 `UPDATE`, `DELETE`, `TRUNCATE`를 거부해요. 서버를 거치지 않고 DB에 직접 접속해도 링크를 바꿀 수 없어요. ([마이그레이션](backend/migrations/0002_links_immutable.sql))

- 링크에는 만료 시각이 없고, 데이터는 서버 밖의 DB(Neon)에 있어서 서버를 재시작·재배포해도 유지돼요.

### 원본 URL 검증과 정규화 저장

서버가 아래 순서로 검사하고, 실패하면 `400 { "error": "<한국어 메시지>" }`로 응답해요. 화면은 이 메시지를 그대로 보여줘요.

1. 앞뒤 공백 제거, 빈 값 거부
2. 2048자 초과 거부
3. URL로 파싱할 수 없으면 거부
4. scheme이 `http`/`https`가 아니면 거부 (`javascript:`, `ftp://`, `mailto:` 등)
5. host가 없으면 거부
6. 이 서비스 자신의 단축 주소(`PUBLIC_ORIGIN`과 같은 origin)는 리디렉션 루프를 막기 위해 거부

통과한 URL은 입력 그대로가 아니라 **`url` crate로 정규화한 문자열로 저장**해요. 저장된 값이 그대로 `Location` 헤더에 들어가기 때문에 안전한 표기여야 해서예요. ([ADR-0005](docs/adr/0005-store-normalized-original-url.md))

- `HTTPS://Example.COM` → `https://example.com/`
- `https://ko.wikipedia.org/wiki/러스트` → `https://ko.wikipedia.org/wiki/%EB%9F%AC%EC%8A%A4%ED%8A%B8`
- 경로, 쿼리, 프래그먼트(`#...`)의 의미는 그대로 유지돼요.

## API

| 요청 | 응답 |
|---|---|
| `POST /api/links` `{ "url": "..." }` | `201 { "number": 14 }` / 검증 실패 `400 { "error": "..." }` |
| `GET /{번호}` | 있으면 `302` + `Location: <원본 URL>`, 없거나 잘못된 번호면 `404` HTML 안내 페이지 |
| `PUT`·`PATCH`·`DELETE`·`POST /{번호}` | `405` (`Allow: GET, HEAD`) |
| `GET /health` | DB 연결 정상 `200`, 실패 `503` |

단축 주소는 화면이 "현재 접속한 origin + `/` + 번호"로 조립해요.

## 배포 구성과 라우팅

```
브라우저 ──HTTPS──▶ Cloudflare Worker (short-url.squeezethelime.workers.dev)
                     ├─ /api/*, /^[0-9]+$/ ──프록시──▶ Render 백엔드 (Axum) ──TLS──▶ Neon Postgres
                     └─ 그 밖의 경로 ──▶ Dioxus 정적 파일 (없는 경로는 index.html)
```

공개 주소는 Cloudflare 하나예요. 화면과 단축 주소가 같은 도메인이라 CORS 설정이 필요 없고, 제출한 주소의 `/번호`가 그대로 리디렉션돼요. ([ADR-0001](docs/adr/0001-cloudflare-worker-as-single-public-origin.md))

| 경로 | 처리하는 곳 | 결과 |
|---|---|---|
| `/` | Worker → 정적 파일 | 단축 주소 생성 화면 |
| `/api/links` | Worker → 백엔드 | 링크 생성 |
| `/123` | Worker → 백엔드 | `302` 리디렉션 또는 `404` HTML |
| `/about` 등 숫자가 아닌 경로 | Worker → 정적 파일(index.html) | Dioxus 라우터의 "페이지를 찾을 수 없어요" 화면 |

- Worker가 정적 파일 처리보다 **먼저** 실행돼요(`run_worker_first = true`). 그래야 `/123`이 index.html로 빠지지 않아요.
- 프록시할 때 `redirect: "manual"`로 백엔드의 `302`를 따라가지 않고 브라우저에 그대로 넘겨요.

### Railway 대신 Render + Neon을 쓴 이유

과제는 Railway를 지정했지만, 작업 중 Railway를 쓸 수 없는 문제가 생겨 같은 역할을 하는 Render(Docker 웹 서비스)로 옮겼어요. Render 무료 Postgres는 30일 뒤 만료돼서 "링크는 영구히 유지된다"는 요건과 맞지 않아, 만료가 없는 Neon 무료 Postgres를 DB로 써요. ([ADR-0006](docs/adr/0006-render-and-neon-instead-of-railway.md))

## 환경변수

실제 값은 저장소에 없어요. 형식은 [`.env.example`](.env.example)을 참고하세요.

| 이름 | 쓰는 곳 | 설명 |
|---|---|---|
| `DATABASE_URL` | 백엔드 | Postgres 연결 문자열. Neon은 `?sslmode=require`를 붙인 **direct**(pooler 아님) 주소 |
| `PORT` | 백엔드 | 바인딩 포트(`0.0.0.0`). Render가 자동으로 넣어줘요. 기본값 `3000` |
| `PUBLIC_ORIGIN` | 백엔드 | 공개 주소(예: `https://short-url.squeezethelime.workers.dev`). 자기 단축 주소 거부에 사용 |
| `STATIC_DIR` | 백엔드(로컬 전용, 선택) | 지정하면 dx 빌드 산출물도 함께 서빙해서 로컬에서 한 주소로 확인할 수 있어요 |
| `BACKEND_ORIGIN` | Worker | 백엔드 공개 주소. [`worker/wrangler.toml`](worker/wrangler.toml)의 `[vars]` |

브라우저 번들에는 어떤 비밀값도 들어가지 않아요. 화면은 같은 도메인의 `/api/links`만 상대경로로 호출해요.

## 로컬 실행

### 준비물

- Rust(rustup)와 `wasm32-unknown-unknown` 타깃
- Dioxus CLI `dx` 0.7
- Docker
- Node.js(Worker를 로컬에서 돌리거나 배포할 때)

```bash
rustup target add wasm32-unknown-unknown
cargo binstall dioxus-cli@0.7.10
```

### 1. DB 준비

Docker로 Postgres 17을 띄워요. 포트는 `5433`이에요. 테이블과 트리거는 서버가 시작할 때 마이그레이션으로 자동 생성돼요.

```bash
docker compose up -d
cp .env.example .env
```

### 2. 화면 빌드

⚠️ dx 0.7.10은 `frontend/` 안에서 실행하면 패닉이 나요. **저장소 최상위에서 `-p frontend`로** 실행하세요.

```bash
dx build -p frontend --platform web --release
```

### 3. 서버 실행

`STATIC_DIR`를 주면 서버 하나로 화면까지 열려요. http://localhost:3000 에 접속하세요.

```bash
STATIC_DIR=target/dx/frontend/release/web/public cargo run -p backend
```

### 테스트

로컬 Postgres가 떠 있어야 해요.

```bash
cargo test -p backend
```

- `backend/tests/api.rs`: 실제 Postgres에 연결한 앱에 HTTP 요청을 보내 검증해요. 생성, 리디렉션, 400, 404, 405, 동시 생성 50개, 재시작 후 유지, 같은 URL 새 번호를 다뤄요.
- `backend/tests/immutability.rs`: 앱을 거치지 않은 SQL `UPDATE`/`DELETE`/`TRUNCATE`가 DB 트리거에 막히는지 확인해요.

## 배포 방법

두 배포는 서로 독립적이에요. ([ADR-0004](docs/adr/0004-split-deploy-pipelines.md))

### 백엔드: Render + Neon

1. Neon에서 프로젝트를 만들어요(Postgres 17). Connection pooling을 끈 **direct** 연결 문자열을 복사해요.
2. Render에서 **New → Blueprint**로 이 저장소를 연결하면 [`render.yaml`](render.yaml)을 읽어요. 이 설정으로 multi-stage [`Dockerfile`](Dockerfile)로 빌드하고, `/health` 헬스체크를 하고, push할 때마다 자동 배포해요.
3. Render 대시보드에서 `DATABASE_URL`(Neon 주소)과 `PUBLIC_ORIGIN`(Cloudflare 공개 주소)을 입력해요.
4. 이후에는 `git push`만 하면 다시 배포돼요. 새 버전이 `/health`에서 `200`을 받아야 트래픽이 넘어가요.

### 화면 + 공개 주소: Cloudflare Workers

Cloudflare 빌드 환경에는 Rust와 `dx`가 없어서 로컬에서 빌드한 뒤 올려요.

```bash
dx build -p frontend --platform web --release
```

```bash
cd worker && npm ci && npx wrangler login && npx wrangler deploy
```

## 확인 시나리오

### 결과

공개 주소에서 확인한 결과예요.

| 시나리오 | 결과 |
|---|---|
| 시크릿 창에서 URL을 입력해 숫자형 주소를 생성하고 복사 | 생성된 `https://short-url.squeezethelime.workers.dev/<번호>`가 표시되고, 복사 버튼을 누르면 "복사됨!"이 잠깐 보여요 |
| 생성한 주소를 다른 브라우저에서 열기 | `302`로 원본 URL에 도착해요. 경로, 쿼리, 프래그먼트가 그대로 유지돼요 |
| 서로 다른 URL 여러 개 동시 생성 | 20개를 동시에 만들어도 번호가 모두 다르고, 각 번호가 자기 원본 URL로 이동해요 (로컬 테스트는 50개) |
| 서버 재배포 뒤 이전 주소 | Render 재배포 전에 만든 `/1`이 여러 번 재배포한 뒤에도 같은 원본 URL로 이동해요 |
| 잘못된 URL | `ftp://`, `javascript:`, 형식 오류, 자기 단축 주소 → `400`과 한국어 메시지 |
| 존재하지 않는 숫자 | 없는 번호, `/0`, `/015`, 범위 초과 → `404` HTML 안내 페이지 |
| 기존 연결을 변경하는 요청 | `PUT`/`PATCH`/`DELETE /<번호>` → `405`, 링크는 그대로 |

### 재현 방법

공개 주소를 인자로 주면 위 시나리오 중 브라우저가 필요 없는 부분(생성 → 302 → 404 → 400 → 405 → 동시 생성)을 자동으로 점검해요.

```bash
./scripts/smoke.sh https://short-url.squeezethelime.workers.dev
```

curl로 하나씩 확인할 수도 있어요.

```bash
curl -i -X POST https://short-url.squeezethelime.workers.dev/api/links -H 'content-type: application/json' -d '{"url":"https://example.com/a?b=1#c"}'
```

```bash
curl -I https://short-url.squeezethelime.workers.dev/14
```

```bash
curl -i -X DELETE https://short-url.squeezethelime.workers.dev/14
```

## 알려진 한계

- **남용 방지가 없어요.** 요청 횟수 제한(rate limit), CAPTCHA, 악성 URL 차단이 없어서 누구나 링크를 무제한으로 만들 수 있어요.
- **첫 요청이 느릴 수 있어요.** Render 무료 플랜은 15분 동안 요청이 없으면 서버가 잠들고, 다음 요청에서 깨어나는 데 수십 초가 걸려요.
- **테스트 링크가 남아 있어요.** 링크는 지울 수 없어서, 개발·검증 중 만든 링크(예: example.com으로 가는 링크, 스모크 스크립트가 만든 링크)가 영구히 남아요. 스모크 스크립트도 실행할 때마다 링크를 20여 개 만들어요.
- **번호가 연속적이지 않을 수 있어요.** 위의 "숫자 발급 방식"을 참고하세요.

## 문서

- [`GLOSSARY.md`](GLOSSARY.md): 원본 URL, 번호, 링크, 단축 주소 용어
- [`docs/adr/`](docs/adr/): 설계 결정 기록
- [`.scratch/numeric-short-url/`](.scratch/numeric-short-url/): 스펙과 작업 티켓
