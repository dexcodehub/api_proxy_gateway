import apiClient from "../apiClient";
import { GLOBAL_CONFIG } from "@/global-config";
import type { UserInfo, UserToken } from "#/entity";

export interface SignInReq {
	username: string;
	password: string;
}

export interface SignUpReq extends SignInReq {}
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
    const envTenant = (import.meta as any).env?.VITE_APP_TENANT_ID;
    if (envTenant) return envTenant as string;
    const stored = localStorage.getItem("tenantId");
    if (stored) return stored;
    const generated = `ui-${Math.random().toString(36).slice(2, 10)}`;
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
    // Map username/password to backend login with tenant
    const tenantId = resolveTenantId();
    ensureCsrfToken();
    const payload = { tenant_id: tenantId, email: data.username, password: data.password };
    const res = await apiClient.post<any>({ url: UserApi.Login, data: payload });
    // Backend returns user info via /auth/login (MeOutput) and sets cookie
    const user: UserInfo = {
        id: res?.user_id || res?.id || "",
        username: data.username,
        nickname: data.username,
        email: data.username,
        avatar: "",
        roles: [],
    } as any;
    const token: UserToken = { accessToken: "session", refreshToken: "" } as any;
    return { ...token, user };
};

const signup = async (data: SignUpReq): Promise<SignInRes> => {
    const tenantId = resolveTenantId();
    ensureCsrfToken();
    const payload = { tenant_id: tenantId, email: data.username, name: data.username, password: data.password };
    const res = await apiClient.post<any>({ url: UserApi.Register, data: payload });
    // Auto-login by calling signin
    return signin({ username: data.username, password: data.password });
};

const logout = () => apiClient.get({ url: UserApi.Logout });
const findById = (id: string) => apiClient.get<UserInfo[]>({ url: `${UserApi.Me}` });

export default {
	signin,
	signup,
	findById,
	logout,
};
