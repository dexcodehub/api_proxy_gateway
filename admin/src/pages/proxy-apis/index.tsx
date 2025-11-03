import proxyApiService, { type ProxyApiModel, type CreateProxyApiInput } from "@/api/services/proxyApis";
import { Button } from "@/ui/button";
import { Card } from "@/ui/card";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/ui/form";
import { Label } from "@/ui/label";
import { Input } from "@/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/ui/select";
import { Switch } from "@/ui/switch";
import { Checkbox } from "@/ui/checkbox";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter } from "@/ui/dialog";
import { Title, Text } from "@/ui/typography";
import { cn } from "@/utils";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { toast } from "sonner";
import { useState } from "react";

// 生成租户 ID：使用标准 UUID v4（后端要求 Uuid 类型）
function genId() {
  const cryptoApi = typeof window !== "undefined" ? (window as any).crypto : undefined;
  if (cryptoApi && typeof cryptoApi.randomUUID === "function") {
    return cryptoApi.randomUUID();
  }
  // 兼容环境：使用简易 UUID v4 生成器
  // 模式 xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = Math.random() * 16 | 0;
    const v = c === "x" ? r : (r & 0x3 | 0x8);
    return v.toString(16);
  });
}

export default function ProxyApisPage() {
  const qc = useQueryClient();
  const [tenant, setTenant] = useState<string>("");
  const [search, setSearch] = useState<string>("");
  const [page, setPage] = useState<number>(1);
  const [pageSize, setPageSize] = useState<number>(10);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [editing, setEditing] = useState<ProxyApiModel | null>(null);

  const { data: list = [], isLoading } = useQuery({
    queryKey: ["proxy-apis", tenant],
    queryFn: () => proxyApiService.listProxyApis(tenant || undefined),
  });

  const createMutation = useMutation({
    mutationFn: (payload: CreateProxyApiInput) => proxyApiService.createProxyApi(payload),
    onSuccess: () => { toast.success("创建成功"); qc.invalidateQueries({ queryKey: ["proxy-apis"] }); },
    onError: (err: any) => { toast.error(err?.message || "创建失败"); },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => proxyApiService.deleteProxyApi(id),
    onSuccess: () => { toast.success("删除成功"); qc.invalidateQueries({ queryKey: ["proxy-apis"] }); },
    onError: (err: any) => { toast.error(err?.message || "删除失败"); },
  });

  const toggleMutation = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) => proxyApiService.updateProxyApi(id, { enabled }),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["proxy-apis"] }); },
    onError: (err: any) => { toast.error(err?.message || "更新失败"); },
  });

  const form = useForm<CreateProxyApiInput>({
    defaultValues: { tenant_id: "", endpoint_url: "/proxy/example", method: "GET", forward_target: "https://jsonplaceholder.typicode.com/posts", require_api_key: false },
  });

  const editForm = useForm<{ endpoint_url: string; method: string; forward_target: string; require_api_key: boolean; enabled: boolean }>({
    defaultValues: { endpoint_url: "", method: "GET", forward_target: "", require_api_key: false, enabled: true },
  });

  const onCreate = async (values: CreateProxyApiInput) => {
    if (!values.tenant_id) values.tenant_id = genId();
    await createMutation.mutateAsync(values);
    form.reset();
  };

  // 确保列表为数组，避免后端返回非数组导致 .filter 报错
  const baseList: ProxyApiModel[] = Array.isArray(list) ? list : [];
  const filtered = baseList.filter((m: ProxyApiModel) => {
    if (!search) return true;
    const q = search.toLowerCase();
    return (
      m.endpoint_url.toLowerCase().includes(q) ||
      m.forward_target.toLowerCase().includes(q) ||
      m.method.toLowerCase().includes(q) ||
      m.tenant_id.toLowerCase().includes(q)
    );
  });

  const total = filtered.length;
  const maxPage = Math.max(1, Math.ceil(total / pageSize));
  const pageSafe = Math.min(page, maxPage);
  const start = (pageSafe - 1) * pageSize;
  const pageData = filtered.slice(start, start + pageSize);

  const toggleSelectAll = (checked: boolean) => {
    const set = new Set<string>(selected);
    if (checked) {
      filtered.forEach((m) => set.add(m.id));
    } else {
      set.clear();
    }
    setSelected(set);
  };

  const toggleSelectOne = (id: string, checked: boolean) => {
    const set = new Set<string>(selected);
    if (checked) set.add(id); else set.delete(id);
    setSelected(set);
  };

  const batchDelete = async () => {
    if (selected.size === 0) return;
    if (!window.confirm(`确认删除选中的 ${selected.size} 条记录？`)) return;
    const ids = Array.from(selected);
    await Promise.all(ids.map((id) => deleteMutation.mutateAsync(id).catch(() => null)));
    setSelected(new Set());
    qc.invalidateQueries({ queryKey: ["proxy-apis"] });
  };

  const openEdit = (m: ProxyApiModel) => {
    setEditing(m);
    editForm.reset({ endpoint_url: m.endpoint_url, method: m.method, forward_target: m.forward_target, require_api_key: m.require_api_key, enabled: m.enabled });
  };

  const submitEdit = async () => {
    if (!editing) return;
    const values = editForm.getValues();
    await proxyApiService.updateProxyApi(editing.id, values);
    toast.success("更新成功");
    setEditing(null);
    qc.invalidateQueries({ queryKey: ["proxy-apis"] });
  };

  return (
    <>
    <div className={cn("p-4 space-y-6")}>
      <Card className="p-4">
        <Title as="h3">代理 API 管理</Title>
        <Text variant="body2" color="secondary">配置被代理的接口规则：入口路径、方法、目标地址与访问控制</Text>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4">
          <div className="md:col-span-1">
            <Form {...form}>
              <form onSubmit={form.handleSubmit(onCreate)} className="space-y-4">
                <FormField name="tenant_id" control={form.control} render={({ field }) => (
                  <FormItem>
                    <FormLabel>租户 ID</FormLabel>
                    <FormControl><Input placeholder="可留空自动生成" {...field} /></FormControl>
                    <FormMessage />
                  </FormItem>
                )} />

                <FormField name="endpoint_url" control={form.control} rules={{ required: "必填" }} render={({ field }) => (
                  <FormItem>
                    <FormLabel>入口路径</FormLabel>
                    <FormControl><Input placeholder="/proxy/posts" {...field} /></FormControl>
                    <FormMessage />
                  </FormItem>
                )} />

                <FormField name="method" control={form.control} rules={{ required: "必填" }} render={({ field }) => (
                  <FormItem>
                    <FormLabel>HTTP 方法</FormLabel>
                    <Select defaultValue={field.value} onValueChange={field.onChange}>
                      <SelectTrigger><SelectValue placeholder="选择方法" /></SelectTrigger>
                      <SelectContent>
                        { ["GET","POST","PUT","DELETE","PATCH","HEAD","OPTIONS"].map(m => <SelectItem key={m} value={m}>{m}</SelectItem>) }
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )} />

                <FormField name="forward_target" control={form.control} rules={{ required: "必填" }} render={({ field }) => (
                  <FormItem>
                    <FormLabel>目标地址</FormLabel>
                    <FormControl><Input placeholder="https://jsonplaceholder.typicode.com/posts" {...field} /></FormControl>
                    <FormMessage />
                  </FormItem>
                )} />

                <FormField name="require_api_key" control={form.control} render={({ field }) => (
                  <FormItem>
                    <FormLabel>需要 API Key</FormLabel>
                    <FormControl><Switch checked={field.value} onCheckedChange={field.onChange} /></FormControl>
                    <FormMessage />
                  </FormItem>
                )} />

                <div className="flex justify-end"><Button type="submit" disabled={createMutation.isPending}>创建</Button></div>
              </form>
            </Form>
          </div>

          <div className="md:col-span-2">
            <div className="flex items-end gap-2">
              <div className="flex-1">
                <Label className="mb-1 block">筛选租户</Label>
                <Input placeholder="租户ID（可选）" value={tenant} onChange={(e) => setTenant(e.target.value)} />
              </div>
              <div className="flex-1">
                <Label className="mb-1 block">搜索</Label>
                <Input placeholder="按路径/方法/目标/租户搜索" value={search} onChange={(e) => { setSearch(e.target.value); setPage(1); }} />
              </div>
              <Button variant="outline" onClick={() => qc.invalidateQueries({ queryKey: ["proxy-apis", tenant] })}>刷新</Button>
              <Button variant="destructive" onClick={batchDelete} disabled={selected.size === 0}>批量删除</Button>
            </div>

            <div className="mt-4 overflow-x-auto">
              {isLoading ? (
                <div className="text-sm text-muted-foreground">加载中...</div>
              ) : (
                <table className="min-w-full text-sm">
                  <thead>
                    <tr className="text-left border-b">
                      <th className="py-2 pr-4"><Checkbox checked={selected.size === filtered.length && filtered.length > 0} onCheckedChange={(v) => toggleSelectAll(!!v)} /></th>
                      <th className="py-2 pr-4">租户</th>
                      <th className="py-2 pr-4">方法</th>
                      <th className="py-2 pr-4">入口路径</th>
                      <th className="py-2 pr-4">目标地址</th>
                      <th className="py-2 pr-4">需密钥</th>
                      <th className="py-2 pr-4">启用</th>
                      <th className="py-2">操作</th>
                    </tr>
                  </thead>
                  <tbody>
                    {pageData.map((m: ProxyApiModel) => (
                      <tr key={m.id} className="border-b">
                        <td className="py-2 pr-4"><Checkbox checked={selected.has(m.id)} onCheckedChange={(v) => toggleSelectOne(m.id, !!v)} /></td>
                        <td className="py-2 pr-4 font-mono break-all">{m.tenant_id}</td>
                        <td className="py-2 pr-4">{m.method}</td>
                        <td className="py-2 pr-4">{m.endpoint_url}</td>
                        <td className="py-2 pr-4 font-mono break-all">{m.forward_target}</td>
                        <td className="py-2 pr-4">{m.require_api_key ? "是" : "否"}</td>
                        <td className="py-2 pr-4">
                          <Switch checked={m.enabled} onCheckedChange={(v) => toggleMutation.mutate({ id: m.id, enabled: v })} />
                        </td>
                        <td className="py-2">
                          <div className="flex gap-2">
                            <Button variant="outline" size="sm" onClick={() => openEdit(m)}>编辑</Button>
                            <Button variant="destructive" size="sm" onClick={() => { if (window.confirm("确认删除该记录？")) deleteMutation.mutate(m.id); }}>删除</Button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>

            <div className="mt-4 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-sm text-muted-foreground">共 {total} 条</span>
                <Select defaultValue={String(pageSize)} onValueChange={(v) => { setPageSize(Number(v)); setPage(1); }}>
                  <SelectTrigger className="w-[100px]"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    {[10,20,50].map((n) => <SelectItem key={n} value={String(n)}>{n}/页</SelectItem>)}
                  </SelectContent>
                </Select>
              </div>
              <div className="flex items-center gap-2">
                <Button variant="outline" size="sm" disabled={pageSafe <= 1} onClick={() => setPage((p) => Math.max(1, p - 1))}>上一页</Button>
                <span className="text-sm">第 {pageSafe} / {maxPage} 页</span>
                <Button variant="outline" size="sm" disabled={pageSafe >= maxPage} onClick={() => setPage((p) => Math.min(maxPage, p + 1))}>下一页</Button>
              </div>
            </div>
          </div>
        </div>
      </Card>
    </div>

    <Dialog open={!!editing} onOpenChange={(open) => { if (!open) setEditing(null); }}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>编辑代理 API</DialogTitle>
        </DialogHeader>
        <Form {...editForm}>
          <form className="space-y-4" onSubmit={(e) => { e.preventDefault(); submitEdit(); }}>
            <FormField name="endpoint_url" control={editForm.control} rules={{ required: "必填" }} render={({ field }) => (
              <FormItem>
                <FormLabel>入口路径</FormLabel>
                <FormControl><Input placeholder="/proxy/posts" {...field} /></FormControl>
                <FormMessage />
              </FormItem>
            )} />
            <FormField name="method" control={editForm.control} rules={{ required: "必填" }} render={({ field }) => (
              <FormItem>
                <FormLabel>HTTP 方法</FormLabel>
                <Select defaultValue={field.value} onValueChange={field.onChange}>
                  <SelectTrigger><SelectValue placeholder="选择方法" /></SelectTrigger>
                  <SelectContent>
                    {["GET","POST","PUT","DELETE","PATCH","HEAD","OPTIONS"].map(m => <SelectItem key={m} value={m}>{m}</SelectItem>)}
                  </SelectContent>
                </Select>
                <FormMessage />
              </FormItem>
            )} />
            <FormField name="forward_target" control={editForm.control} rules={{ required: "必填" }} render={({ field }) => (
              <FormItem>
                <FormLabel>目标地址</FormLabel>
                <FormControl><Input placeholder="https://jsonplaceholder.typicode.com/posts" {...field} /></FormControl>
                <FormMessage />
              </FormItem>
            )} />
            <FormField name="require_api_key" control={editForm.control} render={({ field }) => (
              <FormItem>
                <FormLabel>需要 API Key</FormLabel>
                <FormControl><Switch checked={field.value} onCheckedChange={field.onChange} /></FormControl>
                <FormMessage />
              </FormItem>
            )} />
            <FormField name="enabled" control={editForm.control} render={({ field }) => (
              <FormItem>
                <FormLabel>启用</FormLabel>
                <FormControl><Switch checked={field.value} onCheckedChange={field.onChange} /></FormControl>
                <FormMessage />
              </FormItem>
            )} />
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setEditing(null)}>取消</Button>
              <Button type="submit">保存</Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
    </>
  );
}