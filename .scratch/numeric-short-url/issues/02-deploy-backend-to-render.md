# 02: 백엔드를 Render에 배포 (DB는 Neon)

**What to build:** 백엔드는 Render에서, Postgres는 Neon에서 돌아가고, GitHub에 push하면 자동으로 재배포된다. Render 주소로 링크를 생성하고 이동할 수 있으며, 재배포한 뒤에도 이전 링크가 같은 원본 URL로 이동한다. 배포 플랫폼은 헬스체크로 새 버전이 정상인지 확인한 뒤 트래픽을 넘긴다. (ADR-0004, ADR-0006)

**Blocked by:** 01 (최소 관통: 생성하고 이동하기)

**Status:** ready-for-agent

- [ ] 저장소가 GitHub에 있고 Render 서비스가 그 저장소에 연결되어(`render.yaml`) push마다 배포된다
- [x] multi-stage Dockerfile로 백엔드가 빌드되며, 같은 이미지를 로컬에서도 실행해볼 수 있다
- [x] 백엔드가 `PORT` 환경변수의 포트로 `0.0.0.0`에 바인딩한다
- [ ] `DATABASE_URL`, `PUBLIC_ORIGIN`을 환경변수로 받고, 저장소에는 예시 파일만 있고 실제 비밀값은 없다
- [ ] `GET /health`가 DB 연결을 확인해 정상이면 `200`, 실패하면 `503`을 응답하고, Render 헬스체크 경로로 설정되어 있다
- [ ] Neon Postgres에 TLS로(direct 연결 문자열) 연결되고 서버 시작 시 마이그레이션이 적용된다
- [ ] Render 공개 주소로 `POST /api/links`와 `GET /{번호}` 리디렉션이 동작한다
- [ ] 재배포 후 이전에 만든 번호가 같은 원본 URL로 이동한다
