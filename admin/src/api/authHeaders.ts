import userStore from "@/store/userStore";

/**
 * Build Authorization and CSRF headers from current app state.
 * - Authorization: `Bearer <accessToken>` when available
 * - X-CSRF-Token: value from localStorage when present
 */
export function getAuthHeaders(extra?: Record<string, string>): Record<string, string> {
  const headers: Record<string, string> = { ...(extra || {}) };

  // CSRF
  const csrfToken = typeof window !== "undefined" ? localStorage.getItem("csrfToken") : null;
  if (csrfToken) headers["X-CSRF-Token"] = csrfToken;

  // Authorization
  try {
    const { userToken } = userStore.getState();
    const accessToken = userToken?.accessToken;
    if (accessToken) headers["Authorization"] = `Bearer ${accessToken}`;
  } catch (_) {
    // ignore store access errors in non-react contexts
  }

  return headers;
}

/** Merge existing headers with auth headers (auth takes precedence). */
export function mergeAuthHeaders(base?: Record<string, string>): Record<string, string> {
  return getAuthHeaders(base);
}