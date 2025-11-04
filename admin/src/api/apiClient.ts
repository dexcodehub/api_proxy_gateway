import { GLOBAL_CONFIG } from "@/global-config";
import { t } from "@/locales/i18n";
import userStore from "@/store/userStore";
import axios, { type AxiosRequestConfig, type AxiosError, type AxiosResponse } from "axios";
import { toast } from "sonner";
import type { Result } from "#/api";
import { ResultStatus } from "#/enum";

const axiosInstance = axios.create({
    baseURL: GLOBAL_CONFIG.apiBaseUrl,
    timeout: 50000,
    withCredentials: true,
    headers: { "Content-Type": "application/json;charset=utf-8" },
});

axiosInstance.interceptors.request.use(
    (config) => {
        // Attach CSRF token if available
        const csrfToken = localStorage.getItem("csrfToken");
        if (csrfToken) {
            (config.headers as Record<string, string>)["X-CSRF-Token"] = csrfToken;
        }
        // Attach Authorization token from user store if present
        try {
            const { userToken } = userStore.getState();
            const accessToken = userToken?.accessToken;
            if (accessToken) {
                (config.headers as Record<string, string>)["Authorization"] = `Bearer ${accessToken}`;
            }
        } catch (_) {
            // ignore store access errors in non-react contexts
        }
        return config;
    },
    (error) => Promise.reject(error),
);

axiosInstance.interceptors.response.use(
    (res: AxiosResponse<any>) => {
        if (!res.data) throw new Error(t("sys.api.apiRequestFailed"));
        // Support both unified Result payloads and raw payloads (e.g. auth/login/register)
        const payload = res.data as any;
        if (payload && typeof payload === "object" && "status" in payload) {
            const { status, data, message } = payload as Result<any>;
            if (status === ResultStatus.SUCCESS) {
                return data as any;
            }
            throw new Error(message || t("sys.api.apiRequestFailed"));
        }
        // Raw payload — return directly
        return payload;
    },
    (error: AxiosError<Result>) => {
        const { response, message } = error || {};
        // Prefer backend textual body when available (e.g., Axum returns string for errors)
        const backendMsg = typeof response?.data === "string" ? (response?.data as string) : (response?.data as any)?.message;
        const errMsg = backendMsg || message || t("sys.api.errorMessage");
        toast.error(errMsg, { position: "top-center" });
        if (response?.status === 401) {
            userStore.getState().actions.clearUserInfoAndToken();
        }
        return Promise.reject(error);
    },
);

class APIClient {
	get<T = unknown>(config: AxiosRequestConfig): Promise<T> {
		return this.request<T>({ ...config, method: "GET" });
	}
	post<T = unknown>(config: AxiosRequestConfig): Promise<T> {
		return this.request<T>({ ...config, method: "POST" });
	}
	put<T = unknown>(config: AxiosRequestConfig): Promise<T> {
		return this.request<T>({ ...config, method: "PUT" });
	}
	delete<T = unknown>(config: AxiosRequestConfig): Promise<T> {
		return this.request<T>({ ...config, method: "DELETE" });
	}
	request<T = unknown>(config: AxiosRequestConfig): Promise<T> {
		return axiosInstance.request<any, T>(config);
	}
}

export default new APIClient();
