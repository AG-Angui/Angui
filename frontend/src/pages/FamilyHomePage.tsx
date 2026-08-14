import { Button, Spinner } from "@heroui/react";
import {
  ArrowRight,
  BookOpen,
  ClipboardPlus,
  FileSearch,
  HeartHandshake,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { Link } from "react-router";
import { useEffect, useState } from "react";
import { listCases, type CaseListItem } from "../api/cases";
import { useAuth } from "../auth/useAuth";

const statusLabels: Record<CaseListItem["status"], string> = {
  active: "正在跟进",
  resolved: "已找到",
  closed: "已结束",
};

export function FamilyHomePage() {
  const { token } = useAuth();
  const [cases, setCases] = useState<CaseListItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!token) return;
    let active = true;
    setIsLoading(true);
    listCases(token)
      .then((items) => {
        if (active) setCases(items.filter((item) => item.access_role === "family"));
      })
      .catch(() => {
        if (active) setError("暂时无法读取案件，请稍后重试。");
      })
      .finally(() => {
        if (active) setIsLoading(false);
      });
    return () => {
      active = false;
    };
  }, [token]);

  return (
    <main className="min-h-[calc(100vh-3.5rem)] bg-[#f2f4f3] px-4 py-6 sm:px-6 lg:px-10 lg:py-10">
      <div className="mx-auto max-w-6xl">
        <header className="flex flex-col justify-between gap-6 border-b border-[#d8e3e0] pb-8 md:flex-row md:items-end">
          <div className="max-w-2xl">
            <div className="flex items-center gap-2 text-sm font-semibold text-[#0d5b56]">
              <HeartHandshake size={18} aria-hidden="true" />
              <span>安归 · 家属端</span>
            </div>
            <h1 className="mt-4 text-3xl font-bold tracking-tight text-[#183330] sm:text-4xl">
              说清楚情况，安心看进展
            </h1>
            <p className="mt-3 max-w-xl text-base leading-7 text-[#667a78]">
              我们会把您提供的信息整理成待核对的建案资料。提交后，您只会看到已经审核并适合公开的进展。
            </p>
          </div>
          <div className="flex items-center gap-2 rounded-lg border border-[#cfe0dc] bg-white px-3 py-2 text-sm text-[#49615d]">
            <ShieldCheck size={17} className="text-[#0d5b56]" aria-hidden="true" />
            家属信息仅用于本案协作
          </div>
        </header>

        {error && (
          <div className="mt-6 flex items-center justify-between gap-3 rounded-lg border border-[#f1c7c5] bg-white px-4 py-3 text-sm text-[#9e2b28]" role="alert">
            <span>{error}</span>
            <Button size="sm" variant="ghost" onPress={() => window.location.reload()}>
              <RefreshCw size={15} aria-hidden="true" /> 重试
            </Button>
          </div>
        )}

        <section className="mt-8 grid gap-6 lg:grid-cols-[minmax(0,1fr)_320px]" aria-label="家属求助入口">
          <div className="border border-[#d8e3e0] bg-white p-6 sm:p-8">
            <p className="text-sm font-semibold text-[#1e7d74]">现在最重要的是</p>
            <h2 className="mt-2 text-2xl font-bold text-[#183330] sm:text-3xl">老人走失了？先从已知信息开始。</h2>
            <p className="mt-3 max-w-2xl text-sm leading-6 text-[#667a78]">
              不确定的内容可以标记为“不知道”。先完成关键问题，之后仍然可以补充照片、常去地点和新的线索。
            </p>
            <div className="mt-7 flex flex-wrap gap-3">
              <Link to="/family/intake" className="inline-flex min-h-11 items-center gap-2 rounded-lg bg-[#0d5b56] px-5 py-3 text-base font-semibold text-white transition-colors hover:bg-[#1e7d74] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#0d5b56]">
                <ClipboardPlus size={18} aria-hidden="true" /> 开始求助 <ArrowRight size={17} aria-hidden="true" />
              </Link>
              {cases.length > 0 && (
                <Link to={`/family/cases/${cases[0].id}`} className="inline-flex min-h-11 items-center rounded-lg px-4 py-3 text-base font-semibold text-[#0d5b56] hover:bg-[#eef7f5] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#0d5b56]">
                  查看当前进展
                </Link>
              )}
            </div>
            <div className="mt-7 grid gap-3 border-t border-[#edf1f0] pt-5 text-sm text-[#49615d] sm:grid-cols-3">
              <span>分步问询，随时保存草稿</span>
              <span>提交前由您再次确认</span>
              <span>公开内容经过人工审核</span>
            </div>
          </div>

          <aside className="border border-[#d8e3e0] bg-[#fbfcfc] p-5">
            <h2 className="text-base font-bold text-[#183330]">家属可以做什么</h2>
            <ul className="mt-4 space-y-4 text-sm text-[#49615d]">
              <li className="flex gap-3"><FileSearch size={18} className="mt-0.5 shrink-0 text-[#1e7d74]" aria-hidden="true" /><span>补充或更正老人画像和最后出现信息</span></li>
              <li className="flex gap-3"><ClipboardPlus size={18} className="mt-0.5 shrink-0 text-[#1e7d74]" aria-hidden="true" /><span>上传清晰照片，提交新的线索和常去地点</span></li>
              <li className="flex gap-3"><BookOpen size={18} className="mt-0.5 shrink-0 text-[#1e7d74]" aria-hidden="true" /><span>查看已审核的公开进展与待补信息</span></li>
            </ul>
          </aside>
        </section>

        <section className="mt-10" aria-label="案件列表">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-sm font-semibold text-[#1e7d74]">我的求助</p>
              <h2 className="mt-1 text-xl font-bold text-[#183330]">已有案件</h2>
            </div>
            {isLoading && <Spinner size="sm" aria-label="正在加载案件" />}
          </div>
          {!isLoading && cases.length === 0 ? (
            <div className="mt-4 border border-dashed border-[#bcd1cc] bg-white px-5 py-7 text-sm text-[#667a78]">
              还没有求助记录。开始填写后，离开页面也可以从草稿继续。
            </div>
          ) : (
            <div className="mt-4 grid gap-3 md:grid-cols-2">
              {cases.map((item) => (
                <Link key={item.id} to={`/family/cases/${item.id}`} className="group border border-[#d8e3e0] bg-white p-5 transition-colors hover:border-[#1e7d74] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[#0d5b56]">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <h3 className="text-lg font-bold text-[#183330] group-hover:text-[#0d5b56]">{item.display_name || "待完善的老人信息"}</h3>
                      <p className="mt-1 text-xs text-[#667a78]">案件编号 {item.case_code}</p>
                    </div>
                    <span className="shrink-0 rounded-full border border-[#cfe0dc] px-2.5 py-1 text-xs font-semibold text-[#1e7d74]">{statusLabels[item.status]}</span>
                  </div>
                  <p className="mt-5 text-sm text-[#667a78]">{item.last_seen_location ? `最后出现：${item.last_seen_location}` : "最后出现地点尚未补充"}</p>
                  <span className="mt-4 inline-flex items-center gap-1 text-sm font-semibold text-[#0d5b56]">查看公开进展 <ArrowRight size={15} aria-hidden="true" /></span>
                </Link>
              ))}
            </div>
          )}
        </section>
      </div>
    </main>
  );
}
