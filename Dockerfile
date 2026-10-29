# 1단계: Rust로 백엔드를 빌드해요. (이 단계의 무거운 도구들은 최종 이미지에 안 들어가요)
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
# frontend도 workspace 멤버라 매니페스트는 필요하지만, 빌드는 backend만 해요.
RUN cargo build --release -p backend

# 2단계: 실행에 필요한 바이너리만 작은 이미지에 담아요.
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/backend /usr/local/bin/backend
# PORT, DATABASE_URL 은 Render가 실행할 때 환경변수로 넣어줘요.
ENV PORT=3000
EXPOSE 3000
CMD ["backend"]
