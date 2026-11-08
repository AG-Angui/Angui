import { Button, Input, Spinner } from "@heroui/react";
import { ArrowLeft, Check, CircleAlert, Eye, EyeOff, KeyRound } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { FormEvent, Ref } from "react";
import brandMark from "../../../assets/brand/angui-mark.svg";
import { setPassword } from "../api/accessRequests";
import { ApiClientError } from "../api/client";
import "./PasswordSetupPage.css";

type Phase = "form" | "pending" | "success" | "invalid";
type FieldErrors = { password?: string; confirm?: string };

function readSetupToken() {
  const prefix = "#password-setup=";
  return window.location.hash.startsWith(prefix)
    ? window.location.hash.slice(prefix.length)
    : "";
}

function PasswordField({
  id,
  label,
  value,
  error,
  inputRef,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  error?: string;
  inputRef: Ref<HTMLInputElement>;
  onChange: (value: string) => void;
}) {
  const [visible, setVisible] = useState(false);

  return (
    <div className="password-setup-field">
      <label htmlFor={id}>{label}<span className="password-setup-required">（必填）</span></label>
      <div className="password-setup-control">
        <Input
          id={id}
          ref={inputRef}
          name={id}
          aria-label={label}
          type={visible ? "text" : "password"}
          autoComplete="new-password"
          autoCapitalize="none"
          spellCheck={false}
          required
          fullWidth
          value={value}
          aria-invalid={Boolean(error)}
          aria-describedby={`${id === "setup-password" ? "setup-password-rules" : ""} ${error ? `${id}-error` : ""}`.trim() || undefined}
          onChange={(event) => onChange(event.target.value)}
          placeholder={id === "setup-password" ? "请输入新密码" : "请再次输入新密码"}
        />
        <Button
          type="button"
          variant="ghost"
          isIconOnly
          className="password-setup-toggle password-setup-focus"
          aria-label={`${visible ? "隐藏" : "显示"}${label}`}
          aria-controls={id}
          aria-pressed={visible}
          onPress={() => setVisible((previous) => !previous)}
        >
          {visible ? <EyeOff size={18} aria-hidden="true" /> : <Eye size={18} aria-hidden="true" />}
        </Button>
      </div>
      {error && <p id={`${id}-error`} className="password-setup-error" role="alert">{error}</p>}
    </div>
  );
}

export function PasswordSetupPage() {
  const [token] = useState(readSetupToken);
  const [phase, setPhase] = useState<Phase>(token ? "form" : "invalid");
  const [password, setPasswordValue] = useState("");
  const [confirm, setConfirm] = useState("");
  const [errors, setErrors] = useState<FieldErrors>({});
  const [message, setMessage] = useState("");
  const submitting = useRef(false);
  const passwordInput = useRef<HTMLInputElement>(null);
  const confirmInput = useRef<HTMLInputElement>(null);
  const statusTitle = useRef<HTMLHeadingElement>(null);
  const submitError = useRef<HTMLParagraphElement>(null);

  useEffect(() => {
    if (phase === "success" || phase === "invalid") statusTitle.current?.focus();
  }, [phase]);

  useEffect(() => {
    if (message) submitError.current?.focus();
  }, [message]);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting.current) return;

    const nextErrors: FieldErrors = {};
    const length = [...password].length;
    if (!password) nextErrors.password = "请输入新密码。";
    else if (length < 12 || length > 256) nextErrors.password = "密码长度须为 12–256 个字符。";
    if (!confirm) nextErrors.confirm = "请再次输入新密码。";
    else if (confirm !== password) nextErrors.confirm = "两次输入的密码不一致。";
    setErrors(nextErrors);
    setMessage("");
    if (nextErrors.password || nextErrors.confirm) {
      (nextErrors.password ? passwordInput : confirmInput).current?.focus();
      return;
    }
    if (!token) {
      setPhase("invalid");
      return;
    }

    submitting.current = true;
    setPhase("pending");
    try {
      // Keep the existing API contract; confirmation stays in the browser only.
      await setPassword(token, password);
      setPasswordValue("");
      setConfirm("");
      setPhase("success");
    } catch (error) {
      if (error instanceof ApiClientError && error.status === 401) {
        setPasswordValue("");
        setConfirm("");
        setPhase("invalid");
      } else {
        setPhase("form");
        setMessage(error instanceof ApiClientError ? error.message : "暂时无法设置密码，请稍后重试。");
      }
    } finally {
      submitting.current = false;
    }
  }

  const isForm = phase === "form";
  const pending = phase === "pending";
  const success = phase === "success";

  return (
    <div className="password-setup-page flex min-h-dvh flex-col bg-canvas">
      <header className="password-setup-header h-16 shrink-0 border-b border-separator bg-surface">
        <div className="mx-auto flex h-full max-w-[1128px] items-center justify-between gap-4 px-4 md:px-6">
          <a className="password-setup-brand password-setup-focus" href="/" aria-label="安归首页">
            <img src={brandMark} alt="" width={32} height={32} />安归
          </a>
          <a className="password-setup-back password-setup-focus" href="/">
            <ArrowLeft size={16} aria-hidden="true" />返回登录
          </a>
        </div>
      </header>
      <main className="password-setup-main flex w-full flex-1 px-4" aria-label="密码设置">
        <section className={`password-setup-panel ${isForm ? "" : "password-setup-status"}`} aria-labelledby="setup-title" aria-busy={pending}>
          {isForm ? (
            <>
              <div className="password-setup-intro">
                <div className="password-setup-card-brand" aria-hidden="true">
                  <img src={brandMark} alt="" width={36} height={36} /><span>安归</span>
                </div>
                <h1 id="setup-title">设置登录密码</h1>
                <p>设置密码后，即可使用申请时填写的邮箱登录安归。</p>
              </div>
              <form className="password-setup-form" onSubmit={submit} noValidate aria-label="设置登录密码">
                <PasswordField
                  id="setup-password"
                  label="新密码"
                  inputRef={passwordInput}
                  value={password}
                  error={errors.password}
                  onChange={(value) => {
                    setPasswordValue(value);
                    setErrors((previous) => ({ ...previous, password: undefined }));
                    setMessage("");
                  }}
                />
                <PasswordField
                  id="setup-confirm"
                  label="确认密码"
                  inputRef={confirmInput}
                  value={confirm}
                  error={errors.confirm}
                  onChange={(value) => {
                    setConfirm(value);
                    setErrors((previous) => ({ ...previous, confirm: undefined }));
                    setMessage("");
                  }}
                />
                <div id="setup-password-rules" className="password-setup-rules">
                  <p>密码要求</p>
                  <ul>
                    <li>长度为 12–256 个字符，两次输入须一致。</li>
                    <li>建议使用未在其他平台使用过、不易猜测的密码。</li>
                  </ul>
                </div>
                {message && <p ref={submitError} tabIndex={-1} className="password-setup-submit-error" role="alert">{message}</p>}
                <Button type="submit" variant="primary" fullWidth className="password-setup-submit password-setup-focus">
                  <KeyRound size={18} aria-hidden="true" />保存密码
                </Button>
                <a className="password-setup-back password-setup-bottom-link password-setup-focus" href="/">返回登录</a>
              </form>
            </>
          ) : (
            <>
              <div className="password-setup-status-content" role="status" aria-live="polite" aria-atomic="true">
                <div className="password-setup-status-icon" aria-hidden="true">
                  {pending ? <Spinner size="lg" /> : success ? <Check size={28} /> : <CircleAlert size={28} />}
                </div>
                <h1 id="setup-title" ref={statusTitle} tabIndex={-1}>
                  {pending ? "正在设置密码" : success ? "密码设置成功" : "密码设置链接不可用"}
                </h1>
                <p>{pending ? "正在验证设置链接并保存密码，请稍候。" : success ? "密码已设置，请返回登录，使用申请时填写的邮箱和新密码登录。" : "链接缺失、无效、已过期或已被使用。请使用最新的密码设置邮件；如仍无法设置，请联系组织管理员。"}</p>
              </div>
              <div className="password-setup-status-actions">
                {pending ? (
                  <Button variant="primary" fullWidth isDisabled className="password-setup-submit">
                    <Spinner size="sm" aria-hidden="true" />正在保存…
                  </Button>
                ) : (
                  <a className="password-setup-primary-link password-setup-focus" href="/">返回登录</a>
                )}
                {pending && <p>请勿重复提交。</p>}
              </div>
            </>
          )}
        </section>
      </main>
    </div>
  );
}
