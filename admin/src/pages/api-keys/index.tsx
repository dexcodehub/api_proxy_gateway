import apiKeyService, { type ApiKeyRecord } from "@/api/services/apiKeyService";
import { Button } from "@/ui/button";
import { Card } from "@/ui/card";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/ui/form";
import { Input } from "@/ui/input";
import { Title, Text } from "@/ui/typography";
import { cn } from "@/utils";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

type CreateForm = {
  user: string;
  api_key: string;
  expires_at?: string;
};

export default function ApiKeysPage() {
  const { t } = useTranslation();
  const qc = useQueryClient();

  const { data: keys = [], isLoading } = useQuery({
    queryKey: ["api-keys"],
    queryFn: apiKeyService.listApiKeys,
  });

  const createMutation = useMutation({
    mutationFn: (payload: CreateForm) => apiKeyService.createApiKey(payload),
    onSuccess: () => {
      toast.success(t("common.createSuccess") || "创建成功");
      qc.invalidateQueries({ queryKey: ["api-keys"] });
    },
    onError: (err: any) => {
      toast.error(err?.message || t("common.createFail") || "创建失败");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (user: string) => apiKeyService.deleteApiKey(user),
    onSuccess: () => {
      toast.success(t("common.deleteSuccess") || "删除成功");
      qc.invalidateQueries({ queryKey: ["api-keys"] });
    },
    onError: (err: any) => {
      toast.error(err?.message || t("common.deleteFail") || "删除失败");
    },
  });

  const form = useForm<CreateForm>({
    defaultValues: { user: "", api_key: "", expires_at: "" },
  });

  const onCreate = async (values: CreateForm) => {
    await createMutation.mutateAsync(values);
    form.reset();
  };

  return (
    <div className={cn("p-4 space-y-6")}> 
      <Card className="p-4">
        <Title as="h3">API Key 管理</Title>
        <Text variant="body2" color="secondary">创建、查看、删除您的 API Key，并设置有效期</Text>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onCreate)} className="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
            <FormField
              control={form.control}
              name="user"
              rules={{ required: "请输入用户名" }}
              render={({ field }) => (
                <FormItem>
                  <FormLabel>用户名</FormLabel>
                  <FormControl>
                    <Input placeholder="用户名" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="api_key"
              rules={{ required: "请输入 API Key", minLength: { value: 8, message: "至少 8 位" } }}
              render={({ field }) => (
                <FormItem>
                  <FormLabel>API Key</FormLabel>
                  <FormControl>
                    <Input placeholder="例如：sk-xxxxxxxx" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="expires_at"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>有效期</FormLabel>
                  <FormControl>
                    <Input type="date" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <div className="md:col-span-3 flex justify-end">
              <Button type="submit" disabled={createMutation.isPending}>创建</Button>
            </div>
          </form>
        </Form>
      </Card>

      <Card className="p-4">
        <Title as="h3">当前 API Keys</Title>
        {isLoading ? (
          <div className="text-sm text-muted-foreground">加载中...</div>
        ) : (
          <div className="mt-4 overflow-x-auto">
            <table className="min-w-full text-sm">
              <thead>
                <tr className="text-left border-b">
                  <th className="py-2 pr-4">用户名</th>
                  <th className="py-2 pr-4">API Key</th>
                  <th className="py-2 pr-4">有效期</th>
                  <th className="py-2">操作</th>
                </tr>
              </thead>
              <tbody>
                {keys.map((k: ApiKeyRecord) => {
                  const expired = k.expires_at ? new Date(k.expires_at).getTime() < Date.now() : false;
                  return (
                    <tr key={`${k.user}-${k.api_key}`} className="border-b">
                      <td className="py-2 pr-4">{k.user}</td>
                      <td className="py-2 pr-4 font-mono break-all">{k.api_key}</td>
                      <td className="py-2 pr-4">{k.expires_at || "-"}{expired ? <span className="ml-2 text-red-500">(已过期)</span> : null}</td>
                      <td className="py-2">
                        <Button variant="outline" size="sm" onClick={() => navigator.clipboard.writeText(k.api_key)}>复制</Button>
                        <Button variant="destructive" size="sm" className="ml-2" onClick={() => deleteMutation.mutate(k.user)}>删除</Button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </div>
  );
}