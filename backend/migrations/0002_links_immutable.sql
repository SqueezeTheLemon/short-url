-- 링크는 한 번 만들어지면 바꾸거나 지울 수 없어요.
-- 앱에 수정·삭제 라우트가 없더라도, 누군가 DB에 직접 접속해 바꾸는 것까지 여기서 막아요.
CREATE FUNCTION reject_link_changes() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'links are immutable: % is not allowed', TG_OP;
END;
$$;

CREATE TRIGGER links_immutable
    BEFORE UPDATE OR DELETE ON links
    FOR EACH ROW EXECUTE FUNCTION reject_link_changes();

-- TRUNCATE는 행 단위 트리거를 거치지 않아서 따로 막아요.
CREATE TRIGGER links_no_truncate
    BEFORE TRUNCATE ON links
    FOR EACH STATEMENT EXECUTE FUNCTION reject_link_changes();
