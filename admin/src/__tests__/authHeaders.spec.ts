import { getAuthHeaders, mergeAuthHeaders } from "@/api/authHeaders";
import useUserStore from "@/store/userStore";
import { describe, test, expect, beforeEach } from "vitest";

describe("auth headers builder", () => {
  beforeEach(() => {
    useUserStore.getState().actions.clearUserInfoAndToken();
    localStorage.clear();
  });

  test("returns empty when no token/csrf", () => {
    const h = getAuthHeaders();
    expect(h["Authorization"]).toBeUndefined();
    expect(h["X-CSRF-Token"]).toBeUndefined();
  });

  test("merges base headers and injects Authorization/CSRF", () => {
    useUserStore.getState().actions.setUserToken({ accessToken: "abc", refreshToken: "r" });
    localStorage.setItem("csrfToken", "csrf_123");
    const h = mergeAuthHeaders({ "Content-Type": "multipart/form-data" });
    expect(h["Authorization"]).toBe("Bearer abc");
    expect(h["X-CSRF-Token"]).toBe("csrf_123");
    expect(h["Content-Type"]).toBe("multipart/form-data");
  });
});