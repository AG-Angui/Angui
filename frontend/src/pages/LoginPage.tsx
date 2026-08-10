import { Button, Card, Chip, Input, Spinner } from "@heroui/react";
import {
  Eye,
  EyeOff,
  HeartHandshake,
  LogIn,
  RadioTower,
  ShieldCheck,
  UserRound,
} from "lucide-react";
import { useRef, useState } from "react";
import type { FormEvent } from "react";
import brandMark from "../../../assets/brand/angui-mark.svg";
import { useAuth } from "../auth/useAuth";

type Identity = "family" | "volunteer" | "commander";

const identities: Array<{
  id: Identity;
  label: string;
  description: string;
  icon: typeof HeartHandshake;
}> = [
  { id: "family", label: "家属 / 知情人", description: "发起求助，查看公开进展", icon: HeartHandshake },
  { id: "volunteer", label: "志愿者", description: "接收任务，安全执行", icon: UserRound },
  { id: "commander", label: "指挥人员", description: "审核线索，掌握态势", icon: RadioTower },
];

function validEmail(value: string) {
  return value.trim().includes("@");
}

export function LoginPage() {
  const { login, sessionNotice } = useAuth();
  const [identity, setIdentity] = useState<Identity>("family");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [showPassword, setShowPassword] = useState(false);
  const [emailTouched, setEmailTouched] = useState(false);
  const [passwordTouched, setPasswordTouched] = useState(false);
  const emailRef = useRef<HTMLInputElement>(null);
  const passwordRef = useRef<HTMLInputElement>(null);
  const emailError = emailTouched
    ? !email.trim()
      ? "请输入邮箱地址。"
      : !validEmail(email)
        ? "请输入有效的邮箱地址。"
        : ""
    : "";
  const passwordError = passwordTouched && !password ? "请输入密码。" : "";

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setEmailTouched(true);
    setPasswordTouched(true);
    if (!validEmail(email)) return emailRef.current?.focus();
    if (!password) return passwordRef.current?.focus();
    setSubmitting(true);
    setError("");
    try {
      await login(email.trim(), password);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "登录失败");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <main className="min-h-screen bg-[#f2f4f3] px-4 py-8 text-[#123b39] lg:grid lg:grid-cols-[minmax(0,1fr)_440px] lg:items-center lg:gap-16 lg:px-12">
      <section className="mx-auto max-w-2xl lg:mx-0">
        <div className="flex items-center gap-3">
          <img src={brandMark} alt="安归" className="size-14" />
          <div>
            <h1 className="text-3xl font-semibold">安归</h1>
            <p className="text-sm text-[#667a78]">让家人早点回家</p>
          </div>
        </div>
        <h2 className="mt-10 max-w-xl text-3xl font-semibold leading-tight lg:text-5xl">
          面向失智老人走失搜救的协同入口
        </h2>
        <p className="mt-5 max-w-xl text-base leading-7 text-[#667a78]">
          可靠、克制、清晰地协作。登录后将依据已审核的角色权限直接进入对应工作区。
        </p>
        <div className="mt-9 grid gap-3 sm:grid-cols-3">
          {identities.map(({ id, label, description, icon: Icon }) => (
            <button
              key={id}
              type="button"
              aria-pressed={identity === id}
              onClick={() => setIdentity(id)}
              className={[
                "min-h-32 rounded-xl border p-4 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#0d5b56]",
                identity === id
                  ? "border-[#0d5b56] bg-white shadow-sm"
                  : "border-[#d8e5e2] bg-white/60 hover:bg-white",
              ].join(" ")}
            >
              <Icon size={23} className="text-[#0d5b56]" aria-hidden="true" />
              <strong className="mt-4 block text-sm">{label}</strong>
              <span className="mt-1 block text-xs leading-5 text-[#667a78]">{description}</span>
            </button>
          ))}
        </div>
      </section>

      <Card className="mx-auto mt-9 w-full rounded-xl! border border-[#d8e5e2] bg-white shadow-sm lg:mt-0">
        <Card.Header className="border-b border-[#d8e5e2] px-6 py-5">
          <div className="flex w-full items-start justify-between gap-4">
            <div>
              <Card.Title className="text-lg text-[#123b39]">安归｜身份登录</Card.Title>
              <p className="mt-1 text-sm text-[#667a78]">使用已审核的组织账号安全进入。</p>
            </div>
            <Chip size="sm" variant="soft">
              <ShieldCheck size={14} aria-hidden="true" />
              <Chip.Label>受控访问</Chip.Label>
            </Chip>
          </div>
        </Card.Header>
        <Card.Content className="p-6">
          <form className="space-y-4" noValidate onSubmit={submit}>
            {sessionNotice && <p className="rounded-md border border-amber-200 bg-amber-50 p-3 text-sm" role="status">{sessionNotice}</p>}
            <label className="block text-sm font-semibold text-slate-700" htmlFor="login-email">
              邮箱
              <Input ref={emailRef} id="login-email" type="email" autoComplete="username" autoFocus inputMode="email" value={email} onBlur={() => setEmailTouched(true)} onChange={event => { setEmail(event.target.value); setError(""); }} aria-invalid={emailError ? "true" : undefined} className="mt-1.5 min-h-11" disabled={submitting} fullWidth placeholder="name@example.com" />
            </label>
            {emailError && <p className="text-sm text-red-700" role="alert">{emailError}</p>}
            <label className="block text-sm font-semibold text-slate-700" htmlFor="login-password">
              密码
              <div className="relative mt-1.5">
                <Input ref={passwordRef} id="login-password" type={showPassword ? "text" : "password"} autoComplete="current-password" value={password} onBlur={() => setPasswordTouched(true)} onChange={event => { setPassword(event.target.value); setError(""); }} aria-invalid={passwordError || error ? "true" : undefined} className="min-h-11 pr-12" disabled={submitting} fullWidth />
                <Button type="button" size="sm" variant="ghost" isIconOnly className="absolute right-0.5 top-1/2 size-11 -translate-y-1/2" aria-label={showPassword ? "隐藏密码" : "显示密码"} isDisabled={submitting} onPress={() => setShowPassword(value => !value)}>
                  {showPassword ? <EyeOff size={18} /> : <Eye size={18} />}
                </Button>
              </div>
            </label>
            {passwordError && <p className="text-sm text-red-700" role="alert">{passwordError}</p>}
            {error && <p className="rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700" role="alert">{error}</p>}
            <Button type="submit" variant="primary" fullWidth isDisabled={submitting}>
              {submitting ? <Spinner size="sm" aria-label="正在验证登录信息" /> : <LogIn size={17} aria-hidden="true" />}
              {submitting ? "正在验证" : "登录"}
            </Button>
            <a href="/access-request" className="block py-1 text-center text-sm font-medium text-[#0d5b56]">申请访问</a>
          </form>
        </Card.Content>
      </Card>
    </main>
  );
}
