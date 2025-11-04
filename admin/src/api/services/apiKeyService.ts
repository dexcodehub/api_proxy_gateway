import apiClient from "../apiClient";

export type ApiKeyRecord = {
  user: string;
  api_key: string;
  // Optional metadata managed on frontend for display
  expires_at?: string; // ISO string
  permissions?: string[];
};

export const ApiKeyApi = {
  // Use relative paths so axios baseURL '/api' is applied
  List: "admin/api-keys",
  Create: "admin/api-keys",
  Delete: (user: string) => `admin/api-keys/${encodeURIComponent(user)}`,
};

export function listApiKeys(): Promise<ApiKeyRecord[]> {
  return apiClient.get<ApiKeyRecord[]>({ url: ApiKeyApi.List });
}

export function createApiKey(record: ApiKeyRecord): Promise<{ success: boolean } | any> {
  return apiClient.post({ url: ApiKeyApi.Create, data: record });
}

export function deleteApiKey(user: string): Promise<{ success: boolean } | any> {
  return apiClient.delete({ url: ApiKeyApi.Delete(user) });
}

export default {
  listApiKeys,
  createApiKey,
  deleteApiKey,
};