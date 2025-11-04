import apiClient from "@/api/apiClient";
import useUserStore from "@/store/userStore";
import { describe, test, expect, beforeEach, vi } from "vitest";

// Helper: axios adapter that resolves 2xx and rejects non-2xx to trigger error interceptor
function makeAdapter(status = 200, data: any = { ok: true }) {
  return async (config: any) => {
    const response = {
      data,
      status,
      statusText: status === 200 ? "OK" : "Unauthorized",
      headers: {},
      config,
    } as any;
    if (status >= 200 && status < 300) return response;
    const error: any = new Error(response.statusText || "Error");
    error.response = response;
    error.message = (typeof data === "object" && data?.message) || response.statusText;
    throw error;
  };
}

describe("apiClient interceptors", () => {
  beforeEach(() => {
    // reset store
    useUserStore.getState().actions.clearUserInfoAndToken();
    localStorage.clear();
  });

  test("injects Authorization and CSRF on requests when available", async () => {
    // prepare tokens
    useUserStore.getState().actions.setUserToken({ accessToken: "test_token", refreshToken: "r" });
    localStorage.setItem("csrfToken", "csrf_123");

    const res = await apiClient.get<{ ok: boolean }>({ url: "/admin/api-keys", adapter: makeAdapter(200) });
    expect(res).toEqual({ ok: true });
    // headers are on request config
    const cfg = (apiClient as any); // we can't read internals; validate by issuing another call

    // Use a second call to inspect adapter input
    const adapter = vi.fn(makeAdapter(200));
    await apiClient.get({ url: "/admin/api-keys", adapter });
    const calledConfig = adapter.mock.calls[0][0];
    const h = calledConfig.headers as Record<string, string>;
    expect(h["Authorization"]).toBe("Bearer test_token");
    expect(h["X-CSRF-Token"]).toBe("csrf_123");
  });

  test("clears token on 401 response (token expired)", async () => {
    useUserStore.getState().actions.setUserToken({ accessToken: "expired", refreshToken: "r" });
    const adapter = makeAdapter(401, { message: "unauthorized" });
    await expect(apiClient.get({ url: "/admin/api-keys", adapter })).rejects.toBeTruthy();
    const { userToken } = useUserStore.getState();
    expect(userToken.accessToken).toBeUndefined();
  });

  test("concurrent requests all carry Authorization", async () => {
    useUserStore.getState().actions.setUserToken({ accessToken: "concurrent", refreshToken: "r" });
    const a1 = vi.fn(makeAdapter(200));
    const a2 = vi.fn(makeAdapter(200));
    await Promise.all([
      apiClient.get({ url: "/admin/api-keys", adapter: a1 }),
      apiClient.get({ url: "/admin/proxy-apis", adapter: a2 }),
    ]);
    const h1 = (a1.mock.calls[0][0].headers || {}) as Record<string, string>;
    const h2 = (a2.mock.calls[0][0].headers || {}) as Record<string, string>;
    expect(h1["Authorization"]).toBe("Bearer concurrent");
    expect(h2["Authorization"]).toBe("Bearer concurrent");
  });
});