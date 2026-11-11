import { Button, Input, Spinner } from "@heroui/react";
import { ArrowLeft, ArrowRight, Check, CircleAlert, HeartHandshake, Mail, MailCheck, ShieldCheck, Users } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { createAccessRequest, verifyAccessRequest } from "../api/accessRequests";
import { ApiClientError } from "../api/client";

const roles = [
  { value: "family", label: "家属 / 知情人", description: "提供情况，跟进求助", icon: Users },
  { value: "volunteer", label: "志愿者", description: "参与志愿协作", icon: HeartHandshake },
  { value: "commander", label: "指挥人员", description: "参与搜寻协调", icon: ShieldCheck },
] as const;
type RequestedRole = (typeof roles)[number]["value"];
type VerificationPhase = "pending" | "success" | "invalid" | "unavailable";
const verificationError = "验证链接无效、已过期或已被使用。请重新提交申请。";

function readVerificationToken() {
  const prefix = "#access-verify=";
  return window.location.hash.startsWith(prefix)
    ? window.location.hash.slice(prefix.length)
    : null;
}

function initialRole(): RequestedRole {
  const role = new URLSearchParams(window.location.search).get("role");
  return roles.find((item) => item.value === role)?.value ?? "family";
}

function PageHeader() {
  return (
    <header className="access-header">
      <a className="access-brand access-focus" href="/" aria-label="安归首页">安归<span>申请访问</span></a>
      <a className="access-back access-focus" href="/"><ArrowLeft size={16} aria-hidden="true" />返回登录</a>
    </header>
  );
}

export function AccessRequestPage() {
  // Read without consuming during render: StrictMode replays renders and effects.
  const [verificationToken] = useState(readVerificationToken);
  const [verificationPhase, setVerificationPhase] = useState<VerificationPhase>(verificationToken === "" ? "invalid" : "pending");
  const [verificationMessage, setVerificationMessage] = useState("");
  const [verificationAttempt, setVerificationAttempt] = useState(0);
  const verificationRequest = useRef<ReturnType<typeof verifyAccessRequest> | null>(null);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<RequestedRole>(initialRole);
  const [sent, setSent] = useState(false);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const submitting = useRef(false);
  const nameInput = useRef<HTMLInputElement>(null);
  const emailInput = useRef<HTMLInputElement>(null);
  const [errors, setErrors] = useState<{ name?: string; email?: string }>({});
  const receiptHeading = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    if (verificationToken === null) return;
    window.history.replaceState(window.history.state, "", `${window.location.pathname}${window.location.search}`);
    if (!verificationToken) return;
    const subscription = new AbortController();
    // Share the one-time request across StrictMode's effect cleanup/setup cycle.
    verificationRequest.current ??= verifyAccessRequest(verificationToken);
    void verificationRequest.current.then((result) => {
      if (subscription.signal.aborted) return;
      setVerificationMessage(result.message);
      setVerificationPhase("success");
    }).catch((error: unknown) => {
      if (subscription.signal.aborted) return;
      setVerificationPhase(error instanceof ApiClientError && (error.status === 0 || error.status >= 500) ? "unavailable" : "invalid");
    });
    // Unsubscribe this effect without aborting the shared one-time request.
    return () => subscription.abort();
  }, [verificationToken, verificationAttempt]);

  useEffect(() => {
    if (sent) receiptHeading.current?.focus();
  }, [sent]);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (submitting.current) return;
    const nextErrors: typeof errors = {};
    if (!name.trim()) nextErrors.name = "请输入姓名。";
    else if ([...name.trim()].length > 120) nextErrors.name = "姓名不能超过 120 个字符。";
    if (!email.trim()) nextErrors.email = "请输入邮箱地址。";
    else if (emailInput.current?.validity.typeMismatch || new TextEncoder().encode(email.trim()).length > 320) nextErrors.email = "请输入有效的邮箱地址。";
    setErrors(nextErrors);
    setMessage("");
    if (nextErrors.name || nextErrors.email) {
      (nextErrors.name ? nameInput : emailInput).current?.focus();
      return;
    }
    submitting.current = true;
    setBusy(true);
    try {
      const result = await createAccessRequest({ display_name: name.trim(), email: email.trim(), requested_role: role });
      setMessage(result.message);
      setSent(true);
    } catch (error) {
      setMessage(error instanceof ApiClientError ? error.message : "暂时无法提交申请，请稍后重试。");
    } finally {
      submitting.current = false;
      setBusy(false);
    }
  }

  if (verificationToken !== null) {
    const pending = verificationPhase === "pending";
    const success = verificationPhase === "success";
    const unavailable = verificationPhase === "unavailable";
    return (
      <div className="access-page">
        <PageHeader />
        <main className="access-verification-stage">
          <section className="access-status-panel" aria-labelledby="verification-title" aria-busy={pending}>
            <div className="access-status-icon" aria-hidden="true">{pending ? <Spinner size="lg" /> : success ? <MailCheck size={30} /> : <CircleAlert size={30} />}</div>
            <p className="access-eyebrow">邮箱验证</p>
            <div role="status" aria-live="polite" aria-atomic="true">
              <h1 id="verification-title">{pending ? "正在验证邮箱" : success ? "邮箱验证完成" : unavailable ? "暂时无法完成验证" : "验证链接不可用"}</h1>
              <p className="access-status-description">{pending ? "正在确认验证链接，请稍候…" : success ? verificationMessage : unavailable ? "网络或服务暂时不可用，请检查连接后重试。如果链接已被使用，请留意审核邮件。" : verificationError}</p>
            </div>
            {success && <div className="access-status-note"><ShieldCheck size={18} aria-hidden="true" /><p>申请已进入人工审核。审核通过后，请按邮件提示设置密码，再返回登录。</p></div>}
            <div className="access-status-actions">
              {unavailable && <Button variant="primary" fullWidth onPress={() => { verificationRequest.current = null; setVerificationPhase("pending"); setVerificationAttempt((attempt) => attempt + 1); }}>重试验证</Button>}
              {!pending && !success && <a href="/access-request" className="access-primary-link access-focus">重新申请<ArrowRight size={16} aria-hidden="true" /></a>}
              <a href="/" className={`${success ? "access-primary-link" : "access-back"} access-focus`}>返回登录</a>
            </div>
          </section>
        </main>
      </div>
    );
  }

  return (
    <div className="access-page">
      <PageHeader />
      <main className="access-main">
        <div className="access-layout">
          <section className="access-form-panel" aria-labelledby="access-title">
            {!sent ? <>
              <div className="access-form-intro">
                <p className="access-eyebrow">ACCESS REQUEST · 访问申请</p>
                <h1 id="access-title">申请访问安归</h1>
                <p>填写基本信息，完成邮箱验证后等待人工审核。</p>
              </div>
              <form onSubmit={submit} noValidate aria-label="访问申请" aria-busy={busy}>
                <fieldset className="access-fieldset" disabled={busy}>
                  <legend className="access-section-title"><span>01</span>基本信息</legend>
                  <p className="access-section-hint">以下信息均为必填，邮箱用于验证及接收审核通知。</p>
                  <div className="access-fields">
                    <div className="access-field">
                      <label htmlFor="access-name">姓名<span aria-hidden="true"> *</span></label>
                      <Input id="access-name" aria-label="姓名" ref={nameInput} required autoComplete="name" placeholder="请输入姓名" value={name} aria-invalid={Boolean(errors.name)} aria-describedby={errors.name ? "access-name-error" : undefined} onChange={(event) => { setName(event.target.value); setErrors((previous) => ({ ...previous, name: undefined })); setMessage(""); }} />
                      {errors.name && <p id="access-name-error" className="access-error" role="alert">{errors.name}</p>}
                    </div>
                    <div className="access-field">
                      <label htmlFor="access-email">邮箱<span aria-hidden="true"> *</span></label>
                      <Input id="access-email" aria-label="邮箱" ref={emailInput} required type="email" autoComplete="email" inputMode="email" placeholder="请输入常用邮箱" value={email} aria-invalid={Boolean(errors.email)} aria-describedby={errors.email ? "access-email-error" : undefined} onChange={(event) => { setEmail(event.target.value); setErrors((previous) => ({ ...previous, email: undefined })); setMessage(""); }} />
                      {errors.email && <p id="access-email-error" className="access-error" role="alert">{errors.email}</p>}
                    </div>
                  </div>
                </fieldset>
                <fieldset className="access-fieldset" disabled={busy} aria-describedby="access-role-hint">
                  <legend className="access-section-title"><span>02</span>期望身份</legend>
                  <p id="access-role-hint" className="access-section-hint">请选择符合实际情况的身份，最终权限由管理员审核确定。</p>
                  <div className="access-role-options">
                    {roles.map(({ value, label, description, icon: Icon }) => (
                      <label className="access-role-card" key={value}>
                        <input type="radio" name="requested_role" value={value} checked={role === value} onChange={() => { setRole(value); setMessage(""); }} aria-label={label} aria-describedby={`access-role-${value}`} />
                        <Icon className="access-role-icon" size={22} aria-hidden="true" />
                        <span className="access-role-copy"><span className="access-role-title">{label}</span><span id={`access-role-${value}`} className="access-role-description">{description}</span></span>
                        <span className="access-radio-mark" aria-hidden="true">{role === value && <Check size={12} />}</span>
                      </label>
                    ))}
                  </div>
                </fieldset>
                {message && <p className="access-submit-error" role="alert">{message}</p>}
                <div className="access-submit-area">
                  <p><Mail size={16} aria-hidden="true" />提交后请查收邮箱中的验证链接。</p>
                  <Button type="submit" variant="primary" isDisabled={busy} className="access-submit">{busy ? <Spinner size="sm" aria-hidden="true" /> : <MailCheck size={18} aria-hidden="true" />}{busy ? "正在提交…" : "发送验证邮件"}</Button>
                </div>
              </form>
            </> : <div className="access-receipt">
              <div className="access-status-icon" aria-hidden="true"><Mail size={30} /></div>
              <p className="access-eyebrow">下一步 · 查看邮箱</p>
              <h1 id="access-title" ref={receiptHeading} tabIndex={-1}>申请已提交</h1>
              <p role="status" className="access-status-description">{message}</p>
              <p className="access-receipt-email">申请邮箱：<strong>{email.trim()}</strong></p>
              <div className="access-status-note"><MailCheck size={18} aria-hidden="true" /><p>请检查收件箱与垃圾邮件。完成邮箱验证后，申请才会进入人工审核。</p></div>
              <a className="access-primary-link access-focus" href="/">返回登录<ArrowRight size={16} aria-hidden="true" /></a>
              <Button variant="tertiary" onPress={() => { setSent(false); setMessage(""); }}>修改申请信息</Button>
            </div>}
          </section>
          <aside className="access-guide" aria-labelledby="access-guide-title">
            <div className="access-guide-heading"><ShieldCheck size={28} aria-hidden="true" /><p className="access-eyebrow">申请须知</p><h2 id="access-guide-title">从申请到访问，<br />每一步都有确认。</h2><p>安归通过邮箱验证和人工审核，确认申请身份与访问权限。</p></div>
            <ol className="access-steps" aria-label="访问申请流程">
              <li><span>1</span><div><h3>提交基本信息</h3><p>填写姓名、邮箱与期望身份。</p></div></li>
              <li><span>2</span><div><h3>验证邮箱</h3><p>打开邮件中的验证链接。</p></div></li>
              <li><span>3</span><div><h3>等待人工审核</h3><p>审核通过后，按邮件提示设置密码。</p></div></li>
            </ol>
            <p className="access-guide-note"><ShieldCheck size={16} aria-hidden="true" />申请身份不等于已获权限；提交申请不会直接创建登录会话。</p>
          </aside>
        </div>
      </main>
    </div>
  );
}
