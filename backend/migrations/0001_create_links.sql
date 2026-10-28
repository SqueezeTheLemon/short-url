-- 링크: 번호와 원본 URL의 한 쌍.
-- 번호는 DB 시퀀스가 발급해서 동시 요청에도 겹치지 않아요. (ADR-0002)
CREATE TABLE links (
    number       BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    original_url TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
