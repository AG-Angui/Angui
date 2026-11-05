import { Button, Card, Input, Spinner } from "@heroui/react";
import {
  ArrowDown,
  ArrowRight,
  Check,
  CircleHelp,
  Eye,
  EyeOff,
  HeartHandshake,
  LockKeyhole,
  LogIn,
  RadioTower,
  ShieldCheck,
  UserRound,
} from "lucide-react";
import { useRef, useState } from "react";
import type { FormEvent } from "react";
import { useNavigate } from "react-router";
import brandMark from "../../../assets/brand/angui-mark.svg";
import { ApiClientError } from "../api/client";
import { useAuth } from "../auth/useAuth";

type Identity = "family" | "volunteer" | "commander";

const identities = [
  {
    id: "family",
    label: "家属 / 知情人",
    description: "开始求助，补充线索，查看已确认的进展。",
    action: "登录后开始求助",
    title: "家属 / 知情人登录",
    guidance: "登录后可提交求助信息，或查看与您有关的案件进展。",
    icon: HeartHandshake,
  },
  {
    id: "volunteer",
    label: "志愿者",
    description: "接收本人任务，了解安全要求，反馈现场线索。",
    action: "登录查看任务",
    title: "志愿者登录",
    guidance: "使用已获授权的账号，查看本人任务与必要案情。",
    icon: UserRound,
  },
  {
    id: "commander",
    label: "指挥 / 管理人员",
    description: "协同案件、审核线索，按授权管理人员与权限。",
    action: "登录协同工作区",
    title: "指挥 / 管理人员登录",
    guidance: "使用组织已授权的账号；案件与管理权限分别校验。",
    icon: RadioTower,
  },
] as const;

function validEmail(value: string) {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value.trim());
}

function loginErrorMessage(cause: unknown) {
  if (cause instanceof ApiClientError && cause.status === 401) {
    // Authentication intentionally does not disclose account existence or status.
    return "邮箱或密码错误，请检查后重试。如尚未完成申请，请按下方说明完成邮箱验证、审核和密码设置；账号状态请联系组织管理员核实。";
  }
  if (cause instanceof ApiClientError && cause.status === 403) {
    return "当前请求未获授权，请联系组织管理员核实访问权限。";
  }
  return cause instanceof Error ? cause.message : "登录未完成，请稍后重试。";
}

export function LoginPage() {
  const { login, sessionNotice } = useAuth();
  const navigate = useNavigate();
  const [identity, setIdentity] = useState<Identity>("family");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [retryable, setRetryable] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);
  const [showPassword, setShowPassword] = useState(false);
  const [emailTouched, setEmailTouched] = useState(false);
  const [passwordTouched, setPasswordTouched] = useState(false);
  const emailRef = useRef<HTMLInputElement>(null);
  const passwordRef = useRef<HTMLInputElement>(null);
  const formHeadingRef = useRef<HTMLHeadingElement>(null);
  const currentIdentity = identities.find((item) => item.id === identity)!;
  const emailError = emailTouched
    ? !email.trim()
      ? "请输入邮箱地址。"
      : !validEmail(email)
        ? "请输入有效的邮箱地址。"
        : ""
    : "";
  const passwordError = passwordTouched && !password ? "请输入密码。" : "";

  function selectIdentity(nextIdentity: Identity) {
    if (submittingRef.current) return;
    setIdentity(nextIdentity);
    setShowPassword(false);
  }

  function startFamilyHelp() {
    selectIdentity("family");
    formHeadingRef.current?.focus();
    formHeadingRef.current?.scrollIntoView?.({ block: "start", behavior: "instant" });
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submittingRef.current) return;
    setEmailTouched(true);
    setPasswordTouched(true);
    if (!validEmail(email)) return emailRef.current?.focus();
    if (!password) return passwordRef.current?.focus();
    submittingRef.current = true;
    setSubmitting(true);
    setError("");
    setRetryable(false);
    try {
      await login(email.trim(), password);
      // App resolves this entry using the authenticated account, never the card.
      void navigate("/login", { replace: true });
    } catch (cause) {
      setError(loginErrorMessage(cause));
      setRetryable(
        cause instanceof ApiClientError &&
          (cause.status === 0 || cause.status >= 500),
      );
    } finally {
      submittingRef.current = false;
      setSubmitting(false);
    }
  }

  return (
    <div className="login-page min-h-dvh bg-login-canvas text-login-ink">
      <header className="border-b border-login-line bg-login-surface">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-3 px-4 py-4 sm:px-8 sm:py-5">
          <a href="/" className="login-focus inline-flex min-h-11 items-center gap-3 rounded-lg" aria-label="安归首页">
            <img src={brandMark} alt="" className="size-11 sm:size-12" />
            <div>
              <span className="block text-2xl font-semibold tracking-widest">安归</span>
              <span className="block text-xs text-login-muted">老人走失搜救协同平台</span>
            </div>
          </a>
          <a href="#login-help" className="login-focus inline-flex min-h-11 shrink-0 items-center gap-1.5 rounded-lg px-2 text-sm text-login-muted">
            <CircleHelp size={18} aria-hidden="true" /> 登录帮助
          </a>
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-4 pb-10 pt-7 sm:px-8 sm:pt-10 lg:pt-12">
        <section aria-labelledby="login-introduction">
          <p className="text-sm font-medium text-login-accent">每一份线索，都连着一份牵挂</p>
          <h1 id="login-introduction" className="mt-3 text-3xl font-semibold leading-snug tracking-tight sm:text-4xl lg:text-5xl">
            帮助失联老人<br className="sm:hidden" />尽早平安回家
          </h1>
          <p className="mt-4 max-w-2xl text-sm leading-7 text-login-muted sm:text-base">
            连接家属、志愿者与指挥人员，让求助有人跟进、线索有序汇集、行动安全协同。
          </p>
        </section>

        <section className="mt-6 flex flex-col gap-4 rounded-xl border border-login-line bg-login-soft p-4 sm:flex-row sm:items-center sm:justify-between sm:px-5" aria-label="家属求助入口">
          <div className="flex items-start gap-3">
            <HeartHandshake className="mt-0.5 shrink-0 text-login-accent" size={22} aria-hidden="true" />
            <div>
              <h2 className="font-semibold">家人走失了，需要帮助？</h2>
              <p className="mt-1 text-sm leading-6 text-login-muted">先登录家属账号，再填写已知信息。情况紧急请先拨打 110 报警。</p>
            </div>
          </div>
          <Button variant="secondary" className="min-h-11 shrink-0 border border-login-line bg-login-surface text-login-accent" isDisabled={submitting} onPress={startFamilyHelp}>
            开始求助 <ArrowDown size={16} aria-hidden="true" />
          </Button>
        </section>

        <section className="mt-7 sm:mt-8" aria-labelledby="identity-heading">
          <div className="mb-3 flex flex-wrap items-baseline justify-between gap-2">
            <h2 id="identity-heading" className="text-sm font-semibold">请选择您的身份</h2>
            <p className="text-xs leading-5 text-login-muted">同一账号，按实际授权进入工作区</p>
          </div>
          <div className="grid gap-2 sm:grid-cols-3 sm:gap-4" role="group" aria-label="身份入口">
            {identities.map(({ id, label, description, action, icon: Icon }) => (
              <button
                key={id}
                type="button"
                aria-pressed={identity === id}
                aria-controls="identity-authentication"
                disabled={submitting}
                onClick={() => selectIdentity(id)}
                className={[
                  "login-focus group relative flex min-w-0 items-center gap-3 rounded-xl border px-4 py-3 text-left transition-colors disabled:cursor-wait sm:block sm:p-5",
                  identity === id
                    ? "border-login-accent bg-login-surface shadow-sm"
                    : "border-login-line bg-login-surface hover:border-login-accent",
                ].join(" ")}
              >
                <span className="inline-flex size-10 shrink-0 items-center justify-center rounded-lg bg-login-soft text-login-accent">
                  <Icon size={22} aria-hidden="true" />
                </span>
                <span className="min-w-0 flex-1 sm:mt-4 sm:block">
                  <strong className="block pr-5 text-sm font-semibold sm:text-base">{label}</strong>
                  <span className="mt-1 block text-xs leading-5 text-login-muted sm:min-h-10 sm:text-sm sm:leading-6">{description}</span>
                  <span className="mt-3 hidden items-center gap-1 text-xs font-medium text-login-accent sm:flex">{action}<ArrowRight size={14} aria-hidden="true" /></span>
                </span>
                {identity === id && <Check size={17} className="absolute right-3 top-3 text-login-accent" aria-hidden="true" />}
              </button>
            ))}
          </div>
        </section>

        <div className="mt-6 grid items-start gap-6 sm:mt-8 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.15fr)] lg:gap-12">
          <Card id="identity-authentication" className="min-w-0 gap-0 rounded-2xl border border-login-line bg-login-surface p-0 shadow-sm">
            <Card.Header className="block border-b border-login-line px-5 py-5 sm:px-7">
              <p className="mb-2 flex items-center gap-1.5 text-xs text-login-accent"><LockKeyhole size={14} aria-hidden="true" /> 安归统一身份认证</p>
              <h2 ref={formHeadingRef} tabIndex={-1} className="login-focus scroll-mt-5 rounded text-xl font-semibold">{currentIdentity.title}</h2>
              <p className="mt-2 text-sm leading-6 text-login-muted">{currentIdentity.guidance}</p>
            </Card.Header>
            <Card.Content className="px-5 py-5 sm:px-7 sm:py-6">
              <form className="space-y-5" noValidate onSubmit={submit} aria-label="身份登录" aria-busy={submitting}>
                {sessionNotice && <p className="rounded-lg border border-login-warning-line bg-login-warning-soft p-3 text-sm leading-6 text-login-warning" role="status">{sessionNotice}</p>}
                <div>
                  <label className="block text-sm font-medium" htmlFor="login-email">邮箱</label>
                  <Input
                    ref={emailRef}
                    id="login-email"
                    name="email"
                    type="email"
                    autoComplete="username"
                    inputMode="email"
                    autoCapitalize="none"
                    spellCheck={false}
                    value={email}
                    onBlur={() => setEmailTouched(true)}
                    onChange={(event) => { setEmail(event.target.value); setError(""); setRetryable(false); }}
                    aria-invalid={emailError ? "true" : undefined}
                    aria-describedby={emailError ? "login-email-error" : "login-email-hint"}
                    className="mt-2 min-h-12 w-full border border-login-line bg-login-surface text-base"
                    disabled={submitting}
                    fullWidth
                    placeholder="请输入账号邮箱"
                    required
                  />
                  {emailError
                    ? <p id="login-email-error" className="mt-2 text-sm text-login-danger" role="alert">{emailError}</p>
                    : <p id="login-email-hint" className="mt-2 text-xs leading-5 text-login-muted">使用申请访问时填写的邮箱。</p>}
                </div>
                <div>
                  <label className="block text-sm font-medium" htmlFor="login-password">密码</label>
                  <div className="relative mt-2">
                    <Input
                      ref={passwordRef}
                      id="login-password"
                      name="password"
                      type={showPassword ? "text" : "password"}
                      autoComplete="current-password"
                      value={password}
                      onBlur={() => setPasswordTouched(true)}
                      onChange={(event) => { setPassword(event.target.value); setError(""); setRetryable(false); }}
                      aria-invalid={passwordError ? "true" : undefined}
                      aria-describedby={passwordError ? "login-password-error" : undefined}
                      className="min-h-12 w-full border border-login-line bg-login-surface pr-12 text-base"
                      disabled={submitting}
                      fullWidth
                      placeholder="请输入密码"
                      required
                    />
                    <Button type="button" size="sm" variant="ghost" isIconOnly className="absolute right-0.5 top-1/2 size-11 -translate-y-1/2 text-login-muted" aria-label={showPassword ? "隐藏密码" : "显示密码"} isDisabled={submitting} onPress={() => setShowPassword((value) => !value)}>
                      {showPassword ? <EyeOff size={18} aria-hidden="true" /> : <Eye size={18} aria-hidden="true" />}
                    </Button>
                  </div>
                  {passwordError && <p id="login-password-error" className="mt-2 text-sm text-login-danger" role="alert">{passwordError}</p>}
                </div>
                {error && <p id="login-error" className="rounded-lg border border-login-danger-line bg-login-danger-soft p-3 text-sm leading-6 text-login-danger" role="alert">{error}</p>}
                <Button type="submit" variant="primary" fullWidth className="min-h-12 text-base font-medium" isDisabled={submitting}>
                  {submitting ? <Spinner size="sm" aria-label="正在验证登录信息" /> : <LogIn size={18} aria-hidden="true" />}
                  {submitting ? "正在验证" : retryable ? "重试登录" : "登录"}
                </Button>
                <p className="text-center text-sm leading-6 text-login-muted">还没有账号？ <a href={`/access-request?role=${identity}`} className="login-focus inline-flex min-h-11 items-center rounded px-1 font-medium text-login-accent">申请访问</a></p>
              </form>
            </Card.Content>
          </Card>

          <aside id="login-help" tabIndex={-1} className="login-focus scroll-mt-6 rounded-lg py-1 lg:py-5" aria-labelledby="help-heading">
            <div className="flex items-center gap-2 text-login-accent"><ShieldCheck size={20} aria-hidden="true" /><h2 id="help-heading" className="text-base font-semibold">让协作有序，让牵挂有回应</h2></div>
            <p className="mt-3 max-w-lg text-sm leading-7 text-login-muted">您提供的信息将按案件与角色授权使用。家属查看已确认的进展，志愿者接收本人任务，指挥人员在授权范围内协同处置。</p>
            <div className="mt-6 border-t border-login-line pt-5">
              <h3 className="flex items-center gap-2 text-sm font-semibold"><CircleHelp size={17} aria-hidden="true" /> 登录遇到困难？</h3>
              <dl className="mt-4 space-y-4 text-sm leading-6">
                <div><dt className="font-medium">首次使用 / 申请待审核</dt><dd className="mt-1 text-login-muted">请先申请访问，完成邮箱验证后等待人工审核。收到设置密码邮件后，再返回登录。</dd></div>
                <div><dt className="font-medium">忘记密码 / 账号无法访问</dt><dd className="mt-1 text-login-muted">请联系您申请访问时对接的组织管理员，核实账号状态与权限。请勿向他人提供密码或邮件中的验证链接。</dd></div>
              </dl>
            </div>
          </aside>
        </div>
      </main>
      <footer className="border-t border-login-line px-4 py-5 text-center text-xs leading-6 text-login-muted">安归 · 为每一次平安归来协同努力<br />搜救信息协同工具，不能替代报警与专业救援。</footer>
    </div>
  );
}
