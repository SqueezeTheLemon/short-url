// 공개 주소의 유일한 입구예요 (ADR-0001).
// - `/api/*`와 숫자로만 된 경로(`/123`) → Render 백엔드로 프록시
// - 나머지 → Dioxus 화면(정적 파일, 없는 경로는 index.html)
const NUMBER_PATH = /^\/[0-9]+$/;

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    if (url.pathname.startsWith("/api/") || NUMBER_PATH.test(url.pathname)) {
      return proxyToBackend(request, url, env.BACKEND_ORIGIN);
    }
    return env.ASSETS.fetch(request);
  },
};

async function proxyToBackend(request, url, backendOrigin) {
  const target = new URL(url.pathname + url.search, backendOrigin);
  // redirect: "manual" → 백엔드의 302를 Worker가 따라가지 않고 브라우저에 그대로 넘겨요.
  // (따라가 버리면 원본 사이트의 HTML을 Worker가 받아와 대신 보여주게 돼요)
  return fetch(new Request(target, request), { redirect: "manual" });
}
