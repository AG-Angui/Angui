import { Button, Spinner } from "@heroui/react";
import {
  AlertTriangle,
  CheckCircle2,
  FileImage,
  MapPin,
  MessageCirclePlus,
  RefreshCw,
  ShieldCheck,
  Upload,
} from "lucide-react";
import { Link, useParams } from "react-router";
import type { FormEvent } from "react";
import { useEffect, useMemo, useState } from "react";
import {
  createCasePlace,
  createClue,
  getCase,
  getCasePublicProgress,
  getCaseResourceConfiguration,
  uploadCaseAttachment,
  type CaseDetail,
  type CasePublicProgress,
} from "../api/cases";
import { useAuth } from "../auth/useAuth";

function dateLabel(value: string | null) {
  if (!value) return "未提供";
  return new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium" }).format(new Date(value));
}

export function FamilyCaseProgressPage() {
  const { caseId } = useParams();
  const { token } = useAuth();
  const [detail, setDetail] = useState<CaseDetail | null>(null);
  const [progress, setProgress] = useState<CasePublicProgress | null>(null);
  const [maxBytes, setMaxBytes] = useState(5 * 1024 * 1024);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState("");
  const [clue, setClue] = useState("");
  const [place, setPlace] = useState("");
  const [isSubmittingClue, setIsSubmittingClue] = useState(false);
  const [isSubmittingPlace, setIsSubmittingPlace] = useState(false);
  const [uploadMessage, setUploadMessage] = useState("");

  async function load() {
    if (!token || !caseId) return;
    setIsLoading(true);
    setError("");
    try {
      const [nextDetail, nextProgress, resources] = await Promise.all([
        getCase(token, caseId),
        getCasePublicProgress(token, caseId),
        getCaseResourceConfiguration(token, caseId).catch(() => null),
      ]);
      setDetail(nextDetail);
      setProgress(nextProgress);
      if (resources) setMaxBytes(resources.attachment_max_image_bytes);
    } catch {
      setError("暂时无法读取这份公开进展，请稍后重试。");
    } finally {
      setIsLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // Loading is intentionally tied to the case and session, not form state.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [caseId, token]);

  const visibleAttachments = useMemo(
    () => detail?.attachments.filter((item) => item.review_status === "confirmed" || item.is_own_submission) ?? [],
    [detail],
  );
  const visiblePlaces = useMemo(
    () => detail?.places.filter((item) => item.visibility !== "internal" && (item.review_status === "confirmed" || item.is_own_submission)) ?? [],
    [detail],
  );

  async function submitClue(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!token || !caseId || !clue.trim()) return;
    setIsSubmittingClue(true);
    try {
      await createClue(token, caseId, {
        source: "family",
        source_type: "manual_report",
        content: clue.trim(),
        occurred_at: null,
        location_text: null,
        location_precision: null,
      });
      setClue("");
      await load();
    } catch {
      setError("线索暂时没有提交成功，请检查网络后重试。");
    } finally {
      setIsSubmittingClue(false);
    }
  }

  async function submitPlace(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!token || !caseId || !place.trim()) return;
    setIsSubmittingPlace(true);
    try {
      await createCasePlace(token, caseId, {
        name: place.trim(),
        place_type: "frequent",
        address: place.trim(),
        longitude: null,
        latitude: null,
        visibility: "confirmed",
      });
      setPlace("");
      await load();
    } catch {
      setError("常去地点暂时没有保存成功，请稍后重试。");
    } finally {
      setIsSubmittingPlace(false);
    }
  }

  async function uploadPhoto(file: File) {
    if (!token || !caseId) return;
    setUploadMessage("");
    try {
      await uploadCaseAttachment(token, caseId, file, maxBytes);
      setUploadMessage("照片已提交，审核后会出现在资料中。");
      await load();
    } catch {
      setUploadMessage("照片未提交成功，请使用 JPG 或 PNG，并检查文件大小。");
    }
  }

  if (isLoading) return <main className="grid min-h-[calc(100vh-3.5rem)] place-items-center bg-[#f2f4f3]"><Spinner size="lg" aria-label="正在加载公开进展" /></main>;
  if (!detail || !progress) {
    return <main className="min-h-[calc(100vh-3.5rem)] bg-[#f2f4f3] px-4 py-10"><div className="mx-auto max-w-xl border border-[#d8e3e0] bg-white p-6"><p role="alert" className="text-sm text-[#9e2b28]">{error || "找不到这份家属案件。"}</p><Link className="mt-5 inline-flex text-sm font-semibold text-[#0d5b56]" to="/family">返回家属端</Link></div></main>;
  }

  const canAdd = detail.status === "active";
  return (
    <main className="min-h-[calc(100vh-3.5rem)] bg-[#f2f4f3] px-4 py-5 sm:px-6 lg:px-10 lg:py-8">
      <div className="mx-auto max-w-6xl">
        <header className="flex flex-col justify-between gap-4 border-b border-[#d8e3e0] pb-6 md:flex-row md:items-end">
          <div>
            <Link to="/family" className="text-sm font-semibold text-[#0d5b56]">返回家属端</Link>
            <div className="mt-3 flex flex-wrap items-center gap-3"><h1 className="text-2xl font-bold text-[#183330] sm:text-3xl">{detail.elder_profile.display_name || "待完善的老人资料"}</h1><span className="rounded-full border border-[#cfe0dc] px-2.5 py-1 text-xs font-semibold text-[#1e7d74]">案件 {detail.case_code}</span></div>
            <p className="mt-2 text-sm text-[#667a78]">这里展示已审核的公开进展。内部调度和未核实线索不会在家属端显示。</p>
          </div>
          <Button size="sm" variant="ghost" onPress={() => void load()}><RefreshCw size={15} aria-hidden="true" /> 刷新进展</Button>
        </header>

        {error && <div role="alert" className="mt-5 rounded-lg border border-[#f1c7c5] bg-white px-4 py-3 text-sm text-[#9e2b28]">{error}</div>}

        <section className="mt-6 grid gap-5 lg:grid-cols-[minmax(0,1fr)_340px]">
          <div className="space-y-5">
            <section className="border border-[#d8e3e0] bg-white p-5 sm:p-6" aria-labelledby="public-progress-title">
              <div className="flex items-start justify-between gap-3"><div><p className="text-sm font-semibold text-[#1e7d74]">只看公开信息</p><h2 id="public-progress-title" className="mt-1 text-xl font-bold">公开进展</h2></div><ShieldCheck className="text-[#0d5b56]" size={22} aria-hidden="true" /></div>
              <div className="mt-5 space-y-4">
                {progress.confirmed_progress.length === 0 ? <p className="border border-dashed border-[#bdd1cc] px-4 py-5 text-sm text-[#667a78]">目前还没有新的公开进展。审核完成后会在这里更新。</p> : progress.confirmed_progress.map((item) => <div key={item.clue_id} className="flex gap-3 border-l-2 border-[#1e7d74] pl-4"><CheckCircle2 size={18} className="mt-0.5 shrink-0 text-[#1e7d74]" aria-hidden="true" /><div><p className="m-0 text-sm font-semibold text-[#183330]">已审核的进展更新</p><p className="mt-1 text-xs text-[#667a78]">更新时间：{dateLabel(item.updated_at)}</p></div></div>)}
              </div>
              {progress.safety_and_contact_reminders.length > 0 && <div className="mt-6 border-t border-[#edf1f0] pt-5"><h3 className="flex items-center gap-2 text-sm font-bold"><AlertTriangle size={16} className="text-[#f5a623]" aria-hidden="true" /> 温馨提醒</h3><ul className="mt-2 space-y-1 text-sm leading-6 text-[#667a78]">{progress.safety_and_contact_reminders.map((item) => <li key={item}>{item}</li>)}</ul></div>}
            </section>

            <section className="border border-[#d8e3e0] bg-white p-5 sm:p-6" aria-labelledby="family-follow-up-title">
              <div><p className="text-sm font-semibold text-[#1e7d74]">需要您补充时会标记在这里</p><h2 id="family-follow-up-title" className="mt-1 text-xl font-bold">待补信息</h2></div>
              {progress.requested_family_information.length === 0 ? <p className="mt-5 text-sm text-[#667a78]">当前没有待补信息。新线索仍可以从下方提交。</p> : <ul className="mt-5 space-y-3">{progress.requested_family_information.map((item) => <li key={item.clue_id} className="flex gap-3 rounded-lg bg-[#fff8e9] px-4 py-3 text-sm text-[#75520b]"><AlertTriangle size={17} className="mt-0.5 shrink-0" aria-hidden="true" /><span>请核对一项家属信息（{item.review_status}）</span></li>)}</ul>}
            </section>
          </div>

          <aside className="space-y-5">
            <section className="border border-[#d8e3e0] bg-white p-5" aria-labelledby="elder-summary-title"><h2 id="elder-summary-title" className="text-base font-bold">老人资料摘要</h2><dl className="mt-4 space-y-3 text-sm"><div className="flex justify-between gap-3 border-b border-[#edf1f0] pb-2"><dt className="text-[#667a78]">年龄</dt><dd className="font-semibold">{detail.elder_profile.age ?? "未提供"}</dd></div><div className="flex justify-between gap-3 border-b border-[#edf1f0] pb-2"><dt className="text-[#667a78]">最后出现</dt><dd className="text-right font-semibold">{dateLabel(detail.elder_profile.last_seen_at)}</dd></div><div className="flex justify-between gap-3"><dt className="text-[#667a78]">地点</dt><dd className="max-w-[170px] text-right font-semibold">{detail.elder_profile.last_seen_location || "未提供"}</dd></div></dl></section>
            <section className="border border-[#d8e3e0] bg-white p-5" aria-labelledby="materials-title"><div className="flex items-center justify-between gap-2"><h2 id="materials-title" className="text-base font-bold">资料管理</h2><FileImage size={18} className="text-[#1e7d74]" aria-hidden="true" /></div><p className="mt-2 text-xs leading-5 text-[#667a78]">上传的照片和地点会先进入审核流程，审核前不会作为公开事实。</p><label className="mt-4 flex min-h-11 cursor-pointer items-center justify-center gap-2 border border-dashed border-[#a8c2bc] px-3 py-2 text-sm font-semibold text-[#0d5b56] hover:bg-[#eef7f5]"><Upload size={16} aria-hidden="true" /> 上传照片<input className="sr-only" type="file" accept="image/jpeg,image/png" onChange={(event) => { const file = event.target.files?.[0]; if (file) void uploadPhoto(file); event.target.value = ""; }} /></label>{uploadMessage && <p className="mt-3 text-xs text-[#49615d]" role="status">{uploadMessage}</p>}{visibleAttachments.length > 0 && <ul className="mt-4 space-y-2 text-xs text-[#667a78]">{visibleAttachments.map((item) => <li key={item.id} className="flex justify-between gap-2"><span className="truncate">{item.original_filename}</span><span>{item.review_status === "confirmed" ? "已审核" : "待审核"}</span></li>)}</ul>}</section>
            <section className="border border-[#d8e3e0] bg-white p-5" aria-labelledby="places-title"><h2 id="places-title" className="flex items-center gap-2 text-base font-bold"><MapPin size={17} className="text-[#1e7d74]" aria-hidden="true" /> 常去地点</h2>{visiblePlaces.length > 0 && <ul className="mt-3 space-y-2 text-sm text-[#49615d]">{visiblePlaces.map((item) => <li key={item.id}>{item.name}</li>)}</ul>}{canAdd && <form className="mt-4 flex gap-2" onSubmit={(event) => void submitPlace(event)}><input aria-label="新增常去地点" className="min-w-0 flex-1 rounded-md border border-[#cfe0dc] px-3 py-2 text-sm outline-none focus:border-[#0d5b56]" value={place} onChange={(event) => setPlace(event.target.value)} placeholder="例如：常去公园" /><Button type="submit" size="sm" isDisabled={!place.trim() || isSubmittingPlace}>{isSubmittingPlace ? <Spinner size="sm" /> : "添加"}</Button></form>}</section>
          </aside>
        </section>

        {canAdd && <section className="mt-5 border border-[#bcd1cc] bg-[#eef7f5] p-5 sm:p-6" aria-labelledby="new-clue-title"><div className="flex items-start gap-3"><MessageCirclePlus size={22} className="mt-0.5 shrink-0 text-[#0d5b56]" aria-hidden="true" /><div className="min-w-0 flex-1"><h2 id="new-clue-title" className="text-lg font-bold">补充新线索</h2><p className="mt-1 text-sm leading-6 text-[#49615d]">只写您亲眼看到或可以核对的内容。不确定的信息请直接标明，提交后会进入人工审核。</p><form className="mt-4 flex flex-col gap-3 sm:flex-row" onSubmit={(event) => void submitClue(event)}><textarea aria-label="新线索内容" className="min-h-24 min-w-0 flex-1 rounded-md border border-[#cfe0dc] bg-white px-3 py-2 text-sm outline-none focus:border-[#0d5b56]" value={clue} onChange={(event) => setClue(event.target.value)} placeholder="例如：今天 16:30 在小区南门看到类似衣着……" /><Button type="submit" size="lg" className="shrink-0 bg-[#0d5b56] text-white" isDisabled={!clue.trim() || isSubmittingClue}>{isSubmittingClue ? <Spinner size="sm" /> : "提交线索"}</Button></form></div></div></section>}
      </div>
    </main>
  );
}
