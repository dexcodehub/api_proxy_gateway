import apiClient from "../apiClient";
import { GLOBAL_CONFIG } from "@/global-config";
import type { UserInfo, UserToken } from "#/entity";

// Align with backend docs: login requires email + password + tenant_id
export interface SignInReq {
    email: string;
    password: string;
}

// Align with backend docs: register requires email + name + password + tenant_id
export interface SignUpReq {
    email: string;
    name: string;
    password: string;
}
export type SignInRes = UserToken & { user: UserInfo };

export enum UserApi {
    Login = "/auth/login",
    Register = "/auth/register",
    Logout = "/auth/logout",
    Me = "/auth/me",
}

/**
 * Resolve tenant id for authentication
 * Priority: env -> localStorage -> generate and persist
 */
function resolveTenantId(): string {
    const envTenant = (import.meta as any).env?.VITE_APP_TENANT_ID as string | undefined;
    const stored = localStorage.getItem("tenantId") || undefined;

    // helper: validate UUID v4
    const isValidUuid = (v?: string) => !!v && /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(v);
    // helper: generate UUID v4
    const genUuidV4 = () => {
        const cryptoApi = typeof window !== "undefined" ? (window as any).crypto : undefined;
        if (cryptoApi && typeof cryptoApi.randomUUID === "function") {
            return cryptoApi.randomUUID();
        }
        // fallback generator
        return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
            const r = (Math.random() * 16) | 0;
            const v = c === "x" ? r : (r & 0x3) | 0x8;
            return v.toString(16);
        });
    };

    // prefer env when valid
    if (isValidUuid(envTenant)) return envTenant!;
    // then stored when valid
    if (isValidUuid(stored)) return stored!;

    // otherwise generate, persist, return
    const generated = genUuidV4();
    localStorage.setItem("tenantId", generated);
    return generated;
}

/** Generate or read CSRF token for headers (paired with server-side validation) */
function ensureCsrfToken(): string {
    const stored = localStorage.getItem("csrfToken");
    if (stored) return stored;
    const token = Math.random().toString(36).slice(2) + Date.now().toString(36);
    localStorage.setItem("csrfToken", token);
    return token;
}

const signin = async (data: SignInReq): Promise<SignInRes> => {
    // Attach tenant if backend requires, MSW will ignore
    const tenantId = resolveTenantId();
    ensureCsrfToken();
    const payload = { tenant_id: tenantId, email: data.email, password: data.password };
    // apiClient will unwrap unified Result and return raw data
    const res = await apiClient.post<SignInRes>({ url: UserApi.Login, data: payload });
    return res;
};

const signup = async (data: SignUpReq): Promise<any> => {
    const tenantId = resolveTenantId();
    ensureCsrfToken();
    const payload = { tenant_id: tenantId, email: data.email, name: data.name, password: data.password };
    return apiClient.post<any>({ url: UserApi.Register, data: payload });
};

const logout = () => apiClient.get({ url: UserApi.Logout });
const findById = (id: string) => apiClient.get<UserInfo[]>({ url: `${UserApi.Me}` });

export default {
	signin,
	signup,
	findById,
	logout,
};
