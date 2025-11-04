import { http, HttpResponse } from "msw";

// Simple in-memory store for API keys
type ApiKeyRecord = { user: string; api_key: string; expires_at?: string; permissions?: string[] };
const STORE: ApiKeyRecord[] = [
  { user: "admin@slash.com", api_key: "demo-admin-key" },
  { user: "test@slash.com", api_key: "demo-test-key" },
];

function requireAuth(headers: Headers): HttpResponse | null {
  const auth = headers.get("Authorization") || "";
  if (!auth || !auth.startsWith("Bearer ")) {
    return new HttpResponse("Unauthorized", { status: 401 });
  }
  return null;
}

const listApiKeys = http.get("/api/admin/api-keys", async ({ request }) => {
  const unauthorized = requireAuth(request.headers);
  if (unauthorized) return unauthorized;
  return HttpResponse.json(STORE, { status: 200 });
});

const createApiKey = http.post("/api/admin/api-keys", async ({ request }) => {
  const unauthorized = requireAuth(request.headers);
  if (unauthorized) return unauthorized;
  const body = (await request.json()) as ApiKeyRecord;
  const idx = STORE.findIndex((r) => r.user === body.user);
  if (idx >= 0) {
    STORE[idx] = body;
  } else {
    STORE.push(body);
  }
  return HttpResponse.json({ success: true }, { status: 200 });
});

const deleteApiKey = http.delete("/api/admin/api-keys/:user", async ({ request, params }) => {
  const unauthorized = requireAuth(request.headers);
  if (unauthorized) return unauthorized;
  const user = String(params.user || "");
  const idx = STORE.findIndex((r) => r.user === user);
  if (idx >= 0) {
    STORE.splice(idx, 1);
  }
  return new HttpResponse(null, { status: 200 });
});

export { listApiKeys, createApiKey, deleteApiKey };