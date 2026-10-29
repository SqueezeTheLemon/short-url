#!/usr/bin/env bash
# Seam ③: 배포된 공개 주소에 실제 요청을 보내 전체 경로(Worker 프록시 포함)를 점검해요.
#
#   ./scripts/smoke.sh https://short-url.squeezethelime.workers.dev
#
# ⚠️ 링크는 지울 수 없어서, 실행할 때마다 테스트 링크가 20여 개 영구히 남아요.
set -euo pipefail

BASE="${1:?사용법: $0 <공개 주소>}"
BASE="${BASE%/}"
RUN="$(date +%s)-$$"
CONCURRENT=20
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

pass() { printf '  ✅ %s\n' "$1"; }
fail() { printf '  ❌ %s\n' "$1"; exit 1; }

# POST /api/links → "<상태코드> <본문>"
create() {
    curl -s -m 60 -w ' %{http_code}' -X POST "$BASE/api/links" \
        -H 'content-type: application/json' -d "{\"url\":\"$1\"}"
}
status_of() { curl -s -o /dev/null -m 60 -w '%{http_code}' "$@"; }
location_of() { curl -s -o /dev/null -m 60 -w '%{redirect_url}' "$1"; }
number_in() { sed -n 's/.*"number":\([0-9]*\).*/\1/p' <<<"$1"; }

echo "▶ $BASE"

echo "1. 생성 → 302 리디렉션 (경로·쿼리·프래그먼트 유지)"
original="https://example.com/smoke/$RUN?q=1#frag"
res="$(create "$original")"
[[ "$res" == *" 201" ]] || fail "생성 실패: $res"
number="$(number_in "$res")"
pass "생성: /$number"
[[ "$(status_of "$BASE/$number")" == 302 ]] || fail "/$number 가 302가 아니에요"
[[ "$(location_of "$BASE/$number")" == "$original" ]] || fail "Location이 원본 URL과 달라요"
pass "/$number → 302 $original"

echo "2. 없는 번호·잘못된 번호 → 404"
for path in /9223372036854775807 /0 "/0$number" /99999999999999999999; do
    [[ "$(status_of "$BASE$path")" == 404 ]] || fail "$path 가 404가 아니에요"
done
pass "/9223372036854775807, /0, /0$number, /99999999999999999999 → 404"

echo "3. 잘못된 URL → 400"
for bad in "ftp://example.com" "javascript:alert(1)" "not a url" "$BASE/1"; do
    res="$(create "$bad")"
    [[ "$res" == *" 400" ]] || fail "'$bad' 가 거부되지 않았어요: $res"
done
pass "ftp, javascript:, 형식 오류, 자기 단축 주소 → 400"

echo "4. 기존 링크 변경 요청 → 405, 링크는 그대로"
for method in PUT PATCH DELETE; do
    [[ "$(status_of -X "$method" "$BASE/$number")" == 405 ]] || fail "$method /$number 가 405가 아니에요"
done
[[ "$(location_of "$BASE/$number")" == "$original" ]] || fail "변경 요청 뒤 링크가 달라졌어요"
pass "PUT/PATCH/DELETE /$number → 405, 여전히 $original"

echo "5. 서로 다른 URL $CONCURRENT 개 동시 생성 → 번호 유일, 각자 자기 URL로"
for i in $(seq 1 "$CONCURRENT"); do
    ( url="https://example.com/smoke/$RUN/concurrent/$i"
      res="$(create "$url")"
      [[ "$res" == *" 201" ]] && echo "$(number_in "$res") $url" >"$TMP/$i" ) &
done
wait
[[ "$(cat "$TMP"/* | wc -l | tr -d ' ')" == "$CONCURRENT" ]] || fail "일부 생성이 실패했어요"
[[ "$(cut -d' ' -f1 "$TMP"/* | sort -u | wc -l | tr -d ' ')" == "$CONCURRENT" ]] || fail "번호가 겹쳤어요"
while read -r n url; do
    [[ "$(location_of "$BASE/$n")" == "$url" ]] || fail "/$n 이 자기 URL로 가지 않아요"
done < <(cat "$TMP"/*)
pass "$CONCURRENT 개 모두 다른 번호, 모두 자기 원본 URL로 이동"

echo "🎉 모든 점검 통과"
