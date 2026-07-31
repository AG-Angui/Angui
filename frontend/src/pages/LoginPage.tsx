import { Button, Card, Chip, Input, Spinner } from "@heroui/react";
import { Eye, EyeOff, LogIn, ShieldCheck } from "lucide-react";
import { useRef, useState } from "react";
import type { FormEvent } from "react";
import brandMark from "../../../assets/brand/angui-mark.svg";
import { useAuth } from "../auth/useAuth";

function hasValidEmail(value: string) {
  return value.trim().includes("@");
}

export function LoginPage() {
  const { login, sessionNotice } = useAuth();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isPasswordVisible, setIsPasswordVisible] = useState(false);
  const [isEmailTouched, setIsEmailTouched] = useState(false);
  const [isPasswordTouched, setIsPasswordTouched] = useState(false);
  const emailInputRef = useRef<HTMLInputElement>(null);
  const passwordInputRef = useRef<HTMLInputElement>(null);

  const emailError = isEmailTouched
    ? email.trim()
      ? hasValidEmail(email)
        ? ""
        : "请输入有效的邮箱地址。"
      : "请输入邮箱地址。"
    : "";
  const passwordError = isPasswordTouched && !password ? "请输入密码。" : "";

  function updateEmail(value: string) {
    setEmail(value);
    if (error) setError("");
  }

  function updatePassword(value: string) {
    setPassword(value);
    if (error) setError("");
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setIsEmailTouched(true);
    setIsPasswordTouched(true);

    if (!hasValidEmail(email)) {
      emailInputRef.current?.focus();
      return;
    }
    if (!password) {
      passwordInputRef.current?.focus();
      return;
    }

    setError("");
    setIsSubmitting(true);
    try {
      await login(email.trim(), password);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "登录失败");
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <main className="grid min-h-screen place-items-center bg-canvas px-4 py-8">
      <div className="w-full max-w-sm">
        <div className="mb-5 flex items-center gap-3 px-1">
          <img src={brandMark} alt="安归" className="size-12 rounded-md" />
          <div>
            <h1 className="m-0 text-2xl font-bold text-slate-950">安归</h1>
            <p className="m-0 mt-0.5 text-sm text-slate-500">协同工作台</p>
          </div>
        </div>

        <Card className="rounded-md! border border-slate-200 shadow-none">
          <Card.Header className="border-b border-slate-200 px-5 py-4">
            <div className="flex w-full items-center justify-between gap-4">
              <div>
                <Card.Title className="text-base font-bold text-slate-950">
                  账号登录
                </Card.Title>
                <p className="m-0 mt-1 text-xs text-slate-500">
                  使用已批准的项目账号进入
                </p>
              </div>
              <Chip size="sm" variant="soft">
                <ShieldCheck size={14} aria-hidden="true" />
                <Chip.Label>受控访问</Chip.Label>
              </Chip>
            </div>
          </Card.Header>
          <Card.Content className="p-5">
            <form className="space-y-4" noValidate onSubmit={handleSubmit}>
              {sessionNotice && (
                <div
                  className="rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-950"
                  role="status"
                  aria-live="polite"
                >
                  {sessionNotice}
                </div>
              )}
              <div>
                <label
                  className="mb-1.5 block text-sm font-semibold text-slate-700"
                  htmlFor="login-email"
                >
                  邮箱
                </label>
                <Input
                  ref={emailInputRef}
                  id="login-email"
                  type="email"
                  autoComplete="username"
                  autoFocus
                  inputMode="email"
                  enterKeyHint="next"
                  value={email}
                  onBlur={() => setIsEmailTouched(true)}
                  onChange={(event) => updateEmail(event.target.value)}
                  aria-describedby={
                    emailError ? "login-email-error" : undefined
                  }
                  aria-invalid={emailError ? "true" : undefined}
                  className="min-h-11"
                  disabled={isSubmitting}
                  fullWidth
                  placeholder="name@example.com"
                />
                {emailError && (
                  <p
                    id="login-email-error"
                    className="mt-1.5 text-sm text-red-700"
                    role="alert"
                  >
                    {emailError}
                  </p>
                )}
              </div>
              <div>
                <label
                  className="mb-1.5 block text-sm font-semibold text-slate-700"
                  htmlFor="login-password"
                >
                  密码
                </label>
                <div className="relative">
                  <Input
                    ref={passwordInputRef}
                    id="login-password"
                    type={isPasswordVisible ? "text" : "password"}
                    autoComplete="current-password"
                    enterKeyHint="go"
                    value={password}
                    onBlur={() => setIsPasswordTouched(true)}
                    onChange={(event) => updatePassword(event.target.value)}
                    aria-describedby={
                      passwordError || error
                        ? "login-password-error"
                        : undefined
                    }
                    aria-invalid={passwordError || error ? "true" : undefined}
                    className="min-h-11 pr-12"
                    disabled={isSubmitting}
                    fullWidth
                  />
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    isIconOnly
                    className="absolute right-0.5 top-1/2 size-11 min-h-11 min-w-11 -translate-y-1/2"
                    aria-label={isPasswordVisible ? "隐藏密码" : "显示密码"}
                    isDisabled={isSubmitting}
                    onPress={() => setIsPasswordVisible((visible) => !visible)}
                  >
                    {isPasswordVisible ? (
                      <EyeOff size={18} aria-hidden="true" />
                    ) : (
                      <Eye size={18} aria-hidden="true" />
                    )}
                  </Button>
                </div>
                {passwordError && (
                  <p
                    id="login-password-error"
                    className="mt-1.5 text-sm text-red-700"
                    role="alert"
                  >
                    {passwordError}
                  </p>
                )}
              </div>

              {error && (
                <div
                  id="login-password-error"
                  className="rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700"
                  role="alert"
                >
                  {error}
                </div>
              )}

              <Button
                type="submit"
                variant="primary"
                fullWidth
                isDisabled={isSubmitting}
              >
                {isSubmitting ? (
                  <Spinner size="sm" aria-label="正在验证登录信息" />
                ) : (
                  <LogIn size={17} aria-hidden="true" />
                )}
                {isSubmitting ? "正在验证" : "登录"}
              </Button>
            </form>
          </Card.Content>
        </Card>
      </div>
    </main>
  );
}
