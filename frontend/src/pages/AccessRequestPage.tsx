import { Button, Card, Input, Spinner } from "@heroui/react";
import { ArrowLeft, MailCheck, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { createAccessRequest, verifyAccessRequest } from "../api/accessRequests";

const verificationError = "验证链接无效、已过期或已被使用。请重新提交申请。";

function takeVerificationToken() {
  const prefix = "#access-verify=";
  if (!window.location.hash.startsWith(prefix)) return null;

  const token = window.location.hash.slice(prefix.length);
  window.history.replaceState({}, "", `${window.location.pathname}${window.location.search}`);
  return token || null;
}

export function AccessRequestPage() {
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [role, setRole] = useState("family");
  const [sent, setSent] = useState(false);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const token = takeVerificationToken();
    if (!token) return;

    let cancelled = false;
    setBusy(true);
    setSent(true);
    void verifyAccessRequest(token)
      .then((result) => {
        if (!cancelled) setMessage(result.message);
      })
      .catch(() => {
        if (!cancelled) setMessage(verificationError);
      })
      .finally(() => {
        if (!cancelled) setBusy(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    setBusy(true);
    setMessage("");
    try {
      const result = await createAccessRequest({
        display_name: name,
        email,
        requested_role: role,
      });
      setMessage(result.message);
      setSent(true);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "暂时无法提交申请");
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="min-h-screen bg-[#f2f4f3] px-4 py-8 text-[#123b39]">
      <div className="mx-auto max-w-lg">
        <a className="mb-6 inline-flex items-center gap-2 text-sm text-[#0d5b56]" href="/">
          <ArrowLeft size={16} />返回登录
        </a>
        <Card className="border border-[#d8e5e2] shadow-none">
          <Card.Header className="border-b border-[#d8e5e2] px-6 py-5">
            <div className="flex items-center gap-3">
              <ShieldCheck size={24} className="text-[#0d5b56]" />
              <div>
                <Card.Title>申请访问安归</Card.Title>
                <p className="mt-1 text-sm text-[#667a78]">邮箱验证后由管理员人工审核。</p>
              </div>
            </div>
          </Card.Header>
          <Card.Content className="p-6">
            {!sent ? (
              <form className="space-y-4" onSubmit={submit}>
                <label className="block text-sm font-medium">
                  姓名
                  <Input aria-label="姓名" value={name} onChange={(event) => setName(event.target.value)} className="mt-1" />
                </label>
                <label className="block text-sm font-medium">
                  邮箱
                  <Input aria-label="邮箱" type="email" value={email} onChange={(event) => setEmail(event.target.value)} className="mt-1" />
                </label>
                <label className="block text-sm font-medium">
                  期望身份
                  <select aria-label="期望身份" value={role} onChange={(event) => setRole(event.target.value)} className="mt-1 min-h-11 w-full rounded-md border border-slate-300 bg-white px-3">
                    <option value="family">家属 / 知情人</option>
                    <option value="volunteer">志愿者</option>
                    <option value="commander">指挥人员</option>
                  </select>
                </label>
                <Button type="submit" variant="primary" fullWidth isDisabled={busy}>
                  {busy ? <Spinner size="sm" /> : <MailCheck size={17} />}发送验证邮件
                </Button>
              </form>
            ) : (
              <div className="rounded-md bg-[#eaf6f1] p-4 text-sm" role="status">
                {busy ? "正在验证邮箱，请稍候…" : message || "请打开邮件中的验证链接，完成验证后将进入人工审核。"}
              </div>
            )}
            {message && !sent && <p className="mt-4 text-sm" role="alert">{message}</p>}
          </Card.Content>
        </Card>
      </div>
    </main>
  );
}
