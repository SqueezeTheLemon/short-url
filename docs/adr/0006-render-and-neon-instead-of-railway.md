# 백엔드는 Railway 대신 Render에, DB는 Neon에 둔다

Railway에 문제가 생겨 백엔드 호스팅을 Render 웹 서비스(Docker, 무료 플랜)로 옮긴다. 빌드 방식(저장소의 multi-stage Dockerfile, push마다 자동 배포, `/health` 헬스체크)은 ADR-0004 그대로이고 설정은 `render.yaml`에 둔다. Render 무료 Postgres는 생성 30일 뒤 만료되어 "링크는 영구히 유지된다"는 요건과 맞지 않으므로, 만료가 없는 Neon 무료 Postgres를 쓴다. Neon은 TLS 연결만 받으므로 sqlx에 rustls를 켜고, 마이그레이션의 advisory lock이 커넥션 풀러(PgBouncer)를 거치면 깨질 수 있어 pooled가 아닌 direct 연결 문자열을 쓴다. 이전 ADR의 "Railway"는 이제 Render(서버)·Neon(DB)을 뜻한다.

## Considered Options

- **Render 유료 Postgres**: 같은 플랫폼 내부망이라 TLS가 필요 없지만 월 비용이 들어 기각.
- **Render 무료 Postgres**: 가장 간단하지만 30일 뒤 데이터가 사라져 채점 시점에 링크가 깨질 수 있어 기각.

## Consequences

- Render 무료 웹 서비스는 15분 동안 요청이 없으면 잠들고, 다음 요청에서 깨어나는 데 수십 초가 걸린다. README에 적는다.
- 과제가 Railway를 지정하므로, 플랫폼을 바꾼 사유를 README에 남긴다.
