import apiClient from "@/api/apiClient";

export type ProxyApiModel = {
  id: string;
  tenant_id: string;
  endpoint_url: string;
  method: string;
  forward_target: string;
  require_api_key: boolean;
  enabled: boolean;
  created_at: string;
  updated_at: string;
};

export type CreateProxyApiInput = {
  tenant_id?: string;
  endpoint_url: string;
  method: string;
  forward_target: string;
  require_api_key: boolean;
};

export type UpdateProxyApiInput = Partial<{
  endpoint_url: string;
  method: string;
  forward_target: string;
  require_api_key: boolean;
  enabled: boolean;
}>;

// 使用全局 axios baseURL `/api`，此处不再重复 `/api` 前缀
const base = "admin/proxy-apis";

export default {
  async listProxyApis(tenant_id?: string): Promise<ProxyApiModel[]> {
    const url = tenant_id ? `${base}?tenant_id=${tenant_id}` : base;
    return apiClient.get({ url });
  },
  async createProxyApi(payload: CreateProxyApiInput): Promise<ProxyApiModel> {
    return apiClient.post({ url: base, data: payload });
  },
  async getProxyApi(id: string): Promise<ProxyApiModel> {
    return apiClient.get({ url: `${base}/${id}` });
  },
  async updateProxyApi(id: string, payload: UpdateProxyApiInput): Promise<ProxyApiModel> {
    return apiClient.put({ url: `${base}/${id}`, data: payload });
  },
  async deleteProxyApi(id: string): Promise<void> {
    await apiClient.delete({ url: `${base}/${id}` });
  },
};