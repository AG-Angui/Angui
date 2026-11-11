import { Button, Input, Modal, Spinner } from "@heroui/react";
import {
  ArrowLeft,
  ArrowRight,
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
import { Fragment, useRef, useState, useSyncExternalStore } from "react";
import type { FormEvent } from "react";
import { useNavigate } from "react-router";
import brandMark from "../../../assets/brand/angui-mark.svg";
import { ApiClientError } from "../api/client";
import { useAuth } from "../auth/useAuth";

type Identity = "family" | "volunteer" | "commander";

const identities = {
  family: {
    label: "家属 / 知情人",
    description: "发起求助，补充线索，查看已确认的进展。",
    action: "登录后开始求助",
    title: "家属 / 知情人登录",
    guidance: "登录后可提交求助信息，或查看与您有关的案件进展。",
    icon: HeartHandshake,
  },
  volunteer: {
    label: "志愿者",
    description: "接收本人任务，了解安全要求，反馈现场线索。",
    action: "登录查看任务",
    title: "志愿者登录",
    guidance: "使用已获授权的账号，查看本人任务与必要案情。",
    icon: UserRound,
  },
  commander: {
    label: "指挥 / 管理人员",
    description: "协同案件、审核线索，按授权管理人员与权限。",
    action: "登录协同工作区",
    title: "指挥 / 管理人员登录",
    guidance: "使用组织已授权的账号；案件与管理权限分别校验。",
    icon: RadioTower,
  },
} as const;

const desktopOrder: Identity[] = ["volunteer", "family", "commander"];
const mobileOrder: Identity[] = ["family", "volunteer", "commander"];
const mobileQuery = "(max-width: 767px)";

function subscribeToLayout(onChange: () => void) {
  const query = window.matchMedia?.(mobileQuery);
  query?.addEventListener("change", onChange);
  return () => query?.removeEventListener("change", onChange);
}

function mobileLayoutSnapshot() {
  return window.matchMedia?.(mobileQuery).matches ?? false;
}

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

function EmergencyNotice({ className = "" }: { className?: string }) {
  return (
    <p className={`text-xs leading-6 text-login-muted ${className}`}>
      家人走失、情况紧急？请先拨打 <a href="tel:110" className="login-focus inline-flex min-h-11 items-center rounded px-1 font-semibold text-login-ink">110 报警</a>，再提交求助信息。
    </p>
  );
}

export function LoginPage() {
  const { login, sessionNotice } = useAuth();
  const navigate = useNavigate();
  // Change DOM order as well as visual order, keeping keyboard navigation coherent.
  const isMobile = useSyncExternalStore(subscribeToLayout, mobileLayoutSnapshot, () => false);
  const [identity, setIdentity] = useState<Identity | null>(null);
  const [isAuthOpen, setAuthOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);
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
  const currentIdentity = identities[identity ?? "family"];
  const emailError = emailTouched
    ? !email.trim()
      ? "请输入邮箱地址。"
      : !validEmail(email)
        ? "请输入有效的邮箱地址。"
        : ""
    : "";
  const passwordError = passwordTouched && !password ? "请输入密码。" : "";

  function openAuthentication(nextIdentity: Identity) {
    if (submittingRef.current) return;
    setIdentity(nextIdentity);
    setAuthOpen(true);
  }

  function closeAuthentication() {
    if (submittingRef.current) return;
    setAuthOpen(false);
    // Keep the email for convenience, never carry a password into another entry.
    setPassword("");
    setShowPassword(false);
    setError("");
    setRetryable(false);
    setEmailTouched(false);
    setPasswordTouched(false);
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
    <div className="login-page flex min-h-dvh flex-col bg-login-canvas text-login-ink">
      <header className="h-16 shrink-0 border-b border-login-line bg-login-surface">
        <div className="mx-auto flex h-full max-w-[1128px] items-center justify-between gap-3 px-4 md:px-6">
          <a href="/" className="login-focus inline-flex min-h-11 items-center gap-2 rounded-md" aria-label="安归首页">
            <img src={brandMark} alt="" className="size-9" />
            <span className="text-xl font-semibold tracking-widest text-login-brand">安归</span>
            <span className="hidden border-l border-login-line pl-3 text-xs text-login-muted md:block">老人走失搜救协同平台</span>
          </a>
          <a href="#login-help" onClick={() => setHelpOpen(true)} className="login-focus inline-flex min-h-11 shrink-0 items-center gap-1.5 rounded-md px-2 text-sm text-login-muted">
            <CircleHelp size={18} aria-hidden="true" /> 登录帮助
          </a>
        </div>
      </header>

      <main className="mx-auto w-full max-w-[1128px] flex-1 px-4 pb-8 md:px-6 md:pb-10">
        <section className="mx-auto max-w-[736px] py-8 text-center md:pb-10 md:pt-12" aria-labelledby="login-introduction">
          <h1 id="login-introduction" className="text-2xl font-semibold leading-10 text-login-brand md:text-[26px] md:leading-12"><span className="text-[32px] md:text-[40px]">安归</span>{" · 统一身份入口"}</h1>
        </section>

        <section aria-labelledby="identity-heading">
          <h2 id="identity-heading" className="mb-4 text-sm font-medium text-login-muted md:mb-6 md:text-center">请选择您的身份，继续登录</h2>
          <div className="grid gap-3 md:grid-cols-3 md:gap-6" role="group" aria-label="身份入口">
            {(isMobile ? mobileOrder : desktopOrder).map((id) => {
              const { label, description, action, icon: Icon } = identities[id];
              return (
                <Fragment key={id}>
                  <button
                    type="button"
                    aria-pressed={identity === id}
                    aria-haspopup="dialog"
                    aria-controls={isAuthOpen && identity === id ? "identity-authentication" : undefined}
                    onClick={() => openAuthentication(id)}
                    className={`login-focus login-identity-card group ${identity === id ? "is-selected" : ""}`}
                  >
                    <span className="login-identity-icon"><Icon size={24} aria-hidden="true" /></span>
                    <span className="login-identity-copy">
                      <strong className="block text-base font-semibold leading-6">{label}</strong>
                      <span className="mt-2 block text-sm leading-6 text-login-muted">{description}</span>
                    </span>
                    <span className="login-identity-action text-sm font-medium text-login-accent">
                      <span className="hidden md:inline">{action}</span><ArrowRight size={18} aria-hidden="true" />
                    </span>
                  </button>
                  {id === "family" && <EmergencyNotice className="-mt-1 md:hidden" />}
                </Fragment>
              );
            })}
          </div>
          <p className="mt-5 flex items-start justify-center gap-2 text-xs leading-6 text-login-muted md:mt-6"><ShieldCheck className="mt-1 shrink-0 text-login-accent" size={16} aria-hidden="true" />身份选择仅用于入口引导，工作区以账号实际授权为准。</p>
          <EmergencyNotice className="hidden text-center md:block" />
        </section>

        <details id="login-help" open={helpOpen} onToggle={(event) => setHelpOpen(event.currentTarget.open)} className="login-focus mx-auto mt-6 max-w-[736px] scroll-mt-4 border-t border-login-line pt-2 md:mt-8" tabIndex={-1}>
          <summary className="login-focus min-h-11 cursor-pointer rounded-md py-3 text-sm font-medium text-login-muted">登录帮助与信息保护</summary>
          <dl className="space-y-4 pb-3 pt-2 text-sm leading-6">
            <div><dt className="font-medium">首次使用 / 申请待审核</dt><dd className="mt-1 text-login-muted">请选择身份并申请访问，完成邮箱验证后等待人工审核。收到设置密码邮件后，再返回登录。</dd></div>
            <div><dt className="font-medium">忘记密码 / 账号无法访问</dt><dd className="mt-1 text-login-muted">请联系您申请访问时对接的组织管理员，核实账号状态与权限。请勿向他人提供密码或邮件中的验证链接。</dd></div>
            <div><dt className="font-medium">信息按授权使用</dt><dd className="mt-1 text-login-muted">家属查看已确认的进展，志愿者接收本人任务，指挥人员在授权范围内协同处置。</dd></div>
          </dl>
        </details>
      </main>
      <footer className="border-t border-login-line px-4 py-4 text-center text-xs leading-6 text-login-muted">安归 · 为每一次平安归来协同努力<br />搜救信息协同工具，不能替代报警与专业救援。</footer>

      <Modal.Backdrop
        isOpen={isAuthOpen}
        onOpenChange={(open) => { if (!open) closeAuthentication(); }}
        isDismissable={!submitting}
        isKeyboardDismissDisabled={submitting}
        className="login-page login-modal-backdrop"
      >
        <Modal.Container placement="center" scroll="outside" className="login-modal-container">
          <Modal.Dialog id="identity-authentication" aria-describedby="authentication-guidance" className="login-auth-dialog">
            <Button type="button" variant="ghost" className="login-focus mb-4 min-h-11 self-start rounded-md px-2 text-sm text-login-muted" isDisabled={submitting} onPress={closeAuthentication}>
              <ArrowLeft size={18} aria-hidden="true" /> 返回身份选择
            </Button>
            <Modal.Header className="gap-0 p-0">
              <p className="mb-2 flex items-center gap-1.5 text-xs leading-5 text-login-accent"><LockKeyhole size={14} aria-hidden="true" /> 安归统一身份认证</p>
              <Modal.Heading className="text-xl font-semibold leading-7 text-login-ink">{currentIdentity.title}</Modal.Heading>
              <p id="authentication-guidance" className="mt-2 text-sm leading-6 text-login-muted">{currentIdentity.guidance}</p>
            </Modal.Header>
            <form className="mt-6 space-y-4" noValidate onSubmit={submit} aria-label="身份登录" aria-busy={submitting}>
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
                  className="mt-2 min-h-12 w-full rounded-md border border-login-line bg-login-surface text-base"
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
                    className="min-h-12 w-full rounded-md border border-login-line bg-login-surface pr-12 text-base"
                    disabled={submitting}
                    fullWidth
                    placeholder="请输入密码"
                    required
                  />
                  <Button type="button" size="sm" variant="ghost" isIconOnly className="login-focus absolute right-0.5 top-1/2 size-11 -translate-y-1/2 rounded-md text-login-muted" aria-label={showPassword ? "隐藏密码" : "显示密码"} isDisabled={submitting} onPress={() => setShowPassword((value) => !value)}>
                    {showPassword ? <EyeOff size={18} aria-hidden="true" /> : <Eye size={18} aria-hidden="true" />}
                  </Button>
                </div>
                {passwordError && <p id="login-password-error" className="mt-2 text-sm text-login-danger" role="alert">{passwordError}</p>}
              </div>
              <details className="text-sm leading-6 text-login-muted">
                <summary className="login-focus min-h-11 cursor-pointer rounded-md py-2.5">首次使用或忘记密码？</summary>
                <p className="pb-2">首次使用请先申请访问，完成邮箱验证与人工审核，再通过邮件设置密码。忘记密码或无法访问，请联系组织管理员核实。</p>
              </details>
              {error && <p id="login-error" className="rounded-lg border border-login-danger-line bg-login-danger-soft p-3 text-sm leading-6 text-login-danger" role="alert">{error}</p>}
              <Button type="submit" variant="primary" fullWidth className="login-focus min-h-12 rounded-md text-base font-medium" isDisabled={submitting}>
                {submitting ? <Spinner size="sm" aria-label="正在验证登录信息" /> : <LogIn size={18} aria-hidden="true" />}
                {submitting ? "正在验证" : retryable ? "重试登录" : "登录"}
              </Button>
              <p className="text-center text-sm leading-6 text-login-muted">还没有账号？ <a href={`/access-request?role=${identity ?? "family"}`} className="login-focus inline-flex min-h-11 items-center rounded px-1 font-medium text-login-accent">申请访问</a></p>
            </form>
          </Modal.Dialog>
        </Modal.Container>
      </Modal.Backdrop>
    </div>
  );
}
