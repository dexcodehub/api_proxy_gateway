import type { SignInReq } from "@/api/services/userService";
import { GLOBAL_CONFIG } from "@/global-config";
import { useSignIn } from "@/store/userStore";
import { Button } from "@/ui/button";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/ui/form";
import { Input } from "@/ui/input";
import { cn } from "@/utils";
import { Loader2 } from "lucide-react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { toast } from "sonner";
import { LoginStateEnum, useLoginStateContext } from "./providers/login-provider";

export function LoginForm({ className, ...props }: React.ComponentPropsWithoutRef<"form">) {
	const { t } = useTranslation();
	const [loading, setLoading] = useState(false);
    const navigatge = useNavigate();

	const { loginState, setLoginState } = useLoginStateContext();
	const signIn = useSignIn();

    const form = useForm<SignInReq>({
        defaultValues: {
            email: "",
            password: "",
        },
    });

	if (loginState !== LoginStateEnum.LOGIN) return null;

    const handleFinish = async (values: SignInReq) => {
        setLoading(true);
        try {
            await signIn(values);
            navigatge(GLOBAL_CONFIG.defaultRoute, { replace: true });
            toast.success(t("sys.login.loginSuccessTitle"), {
                closeButton: true,
            });
        } finally {
            setLoading(false);
        }
    };

	return (
		<div className={cn("flex flex-col gap-6", className)}>
			<Form {...form} {...props}>
				<form onSubmit={form.handleSubmit(handleFinish)} className="space-y-4">
					<div className="flex flex-col items-center gap-2 text-center">
						<h1 className="text-2xl font-bold">{t("sys.login.signInFormTitle")}</h1>
						<p className="text-balance text-sm text-muted-foreground">{t("sys.login.signInFormDescription")}</p>
					</div>

                    <FormField
                        control={form.control}
                        name="email"
                        rules={{
                            required: t("sys.login.emaildPlaceholder"),
                            pattern: { value: /^[^\s@]+@[^\s@]+\.[^\s@]+$/, message: t("sys.login.emaildPlaceholder") },
                        }}
                        render={({ field }) => (
                            <FormItem>
                                <FormLabel>{t("sys.login.email")}</FormLabel>
                                <FormControl>
                                    <Input placeholder={t("sys.login.emaildPlaceholder") || "请输入邮箱"} {...field} />
                                </FormControl>
                                <FormMessage />
                            </FormItem>
                        )}
                    />

                    <FormField
                        control={form.control}
                        name="password"
                        rules={{ required: t("sys.login.passwordPlaceholder"), minLength: { value: 8, message: t("sys.login.passwordPlaceholder") } }}
                        render={({ field }) => (
                            <FormItem>
                                <FormLabel>{t("sys.login.password")}</FormLabel>
                                <FormControl>
                                    <Input type="password" placeholder={t("sys.login.passwordPlaceholder") || "至少8位密码"} {...field} suppressHydrationWarning />
                                </FormControl>
                                <FormMessage />
                            </FormItem>
                        )}
                    />

                    {/* 仅保留基础登录，无记住我与忘记密码 */}

					{/* 登录按钮 */}
					<Button type="submit" className="w-full">
						{loading && <Loader2 className="animate-spin mr-2" />}
						{t("sys.login.loginButton")}
					</Button>

                    {/* 移除手机登录/二维码登录 */}

                    {/* 移除所有第三方登录选项 */}

					{/* 注册 */}
					<div className="text-center text-sm">
						{t("sys.login.noAccount")}
						<Button variant="link" className="px-1" onClick={() => setLoginState(LoginStateEnum.REGISTER)}>
							{t("sys.login.signUpFormTitle")}
						</Button>
					</div>
				</form>
			</Form>
		</div>
	);
}

export default LoginForm;
