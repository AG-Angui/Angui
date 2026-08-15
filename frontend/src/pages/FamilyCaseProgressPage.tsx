import { Button, Spinner } from "@heroui/react";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronDown,
  FileImage,
  Image as ImageIcon,
  MapPin,
  MessageCirclePlus,
  RefreshCw,
  Send,
  ShieldCheck,
  Upload,
  X,
} from "lucide-react";
import { Link, useParams } from "react-router";
import type { FormEvent, ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import {
  createCasePlace,
  createClue,
  downloadCaseAttachment,
  getCase,
  getCasePublicProgress,
  getCaseResourceConfiguration,
  updateElderProfile,
  uploadCaseAttachment,
  type CaseAttachment,
  type CaseDetail,
  type CasePublicProgress,
  type CaseResourceConfiguration,
  type CreateCasePlacePayload,
  type LocationPrecision,
  type PublicClueSourceType,
  type UpdateElderProfilePayload,
} from "../api/cases";
import { useAuth } from "../auth/useAuth";
import { LocationConfirmationPicker } from "../components/LocationConfirmationPicker";
import { parseOptionalCoordinatePair } from "../coordinateInput";

const defaultResources: CaseResourceConfiguration = {
  attachment_max_image_bytes: 5 * 1024 * 1024,
  attachment_max_per_case: 10,
  case_place_types: ["frequent"],
};

const placeTypeLabels: Record<string, string> = {
  frequent: "常去地点",
  key_location: "关键地点",
  last_seen_context: "最后出现相关",
};

type PlaceDraft = Omit<CreateCasePlacePayload, "longitude" | "latitude"> & {
  longitude: string;
  latitude: string;
};

function dateLabel(value: string | null) {
  if (!value) return "未提供";
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium" }).format(date);
}

function toDateTimeLocal(value: string | null) {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  return new Date(date.getTime() - date.getTimezoneOffset() * 60_000)
    .toISOString()
    .slice(0, 16);
}

function toIsoOrNull(value: string) {
  if (!value) return null;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date.toISOString();
}

function nullable(value: string) {
  const normalized = value.trim();
  return normalized || null;
}

function formatBytes(value: number) {
  return value >= 1024 * 1024
    ? `${(value / (1024 * 1024)).toFixed(1)} MiB`
    : `${value} 字节`;
}

function attachmentStatus(item: CaseAttachment) {
  return item.review_status === "confirmed" ? "已审核" : "待人工审核";
}

function Field({
  label,
  required,
  children,
}: {
  label: string;
  required?: boolean;
  children: ReactNode;
}) {
  return (
    <label className="block min-w-0">
      <span className="mb-1.5 block text-sm font-semibold text-[#49615d]">
        {label}
        {required ? " *" : ""}
      </span>
      {children}
    </label>
  );
}

const inputClass =
  "min-h-11 w-full rounded-md border border-[#cfe0dc] bg-white px-3 py-2 text-sm text-[#183330] outline-none transition focus:border-[#0d5b56] focus:ring-2 focus:ring-[#dff0ec] disabled:cursor-not-allowed disabled:bg-slate-100";

type ProfileDraft = {
  display_name: string;
  age: string;
  gender: string;
  physical_description: string;
  clothing_description: string;
  health_notes: string;
  last_seen_at: string;
  last_seen_location: string;
};

function profileDraft(profile: CaseDetail["elder_profile"]): ProfileDraft {
  return {
    display_name: profile.display_name,
    age: profile.age?.toString() ?? "",
    gender: profile.gender ?? "",
    physical_description: profile.physical_description ?? "",
    clothing_description: profile.clothing_description ?? "",
    health_notes: profile.health_notes ?? "",
    last_seen_at: toDateTimeLocal(profile.last_seen_at),
    last_seen_location: profile.last_seen_location ?? "",
  };
}

function ElderProfileEditor({
  detail,
  isSaving,
  onSave,
}: {
  detail: CaseDetail;
  isSaving: boolean;
  onSave: (payload: UpdateElderProfilePayload) => Promise<void>;
}) {
  const [draft, setDraft] = useState(() => profileDraft(detail.elder_profile));
  const [ageError, setAgeError] = useState("");

  useEffect(() => {
    setDraft(profileDraft(detail.elder_profile));
  }, [detail.elder_profile, detail.id, detail.updated_at]);

  function update(key: keyof ProfileDraft, value: string) {
    setAgeError("");
    setDraft((current) => ({ ...current, [key]: value }));
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const age = draft.age.trim() === "" ? undefined : Number(draft.age);
    if (age !== undefined && (!Number.isInteger(age) || age < 0 || age > 130)) {
      setAgeError("年龄必须是 0 到 130 之间的整数。");
      return;
    }
    await onSave({
      display_name: draft.display_name.trim(),
      age,
      gender: nullable(draft.gender) ?? "",
      physical_description: nullable(draft.physical_description) ?? "",
      clothing_description: nullable(draft.clothing_description) ?? "",
      health_notes: nullable(draft.health_notes) ?? "",
      last_seen_at: toIsoOrNull(draft.last_seen_at) ?? "",
      last_seen_location: nullable(draft.last_seen_location) ?? "",
    });
  }

  return (
    <form className="grid gap-4 p-5 sm:p-6" onSubmit={(event) => void submit(event)}>
      <div>
        <h3 className="text-lg font-bold text-[#183330]">人物摘要补充 / 更正</h3>
        <p className="mt-1 text-sm text-[#667a78]">
          每次保存都会保留前后版本；案件状态、成员和任务信息不在此表单中。
        </p>
      </div>
      {ageError && <p role="alert" className="rounded-md border border-[#f1c7c5] bg-white px-3 py-2 text-sm text-[#9e2b28]">{ageError}</p>}
      <div className="grid gap-4 md:grid-cols-2">
        <Field label="姓名"><input aria-label="姓名" className={inputClass} value={draft.display_name} maxLength={120} onChange={(event) => update("display_name", event.target.value)} /></Field>
        <Field label="年龄"><input aria-label="年龄" className={inputClass} type="number" min="0" max="130" value={draft.age} onChange={(event) => update("age", event.target.value)} /></Field>
        <Field label="性别"><input aria-label="性别" className={inputClass} value={draft.gender} maxLength={40} onChange={(event) => update("gender", event.target.value)} /></Field>
        <Field label="最后出现地点"><input aria-label="最后出现地点" className={inputClass} value={draft.last_seen_location} maxLength={500} onChange={(event) => update("last_seen_location", event.target.value)} /></Field>
        <Field label="最后出现时间"><input aria-label="最后出现时间" className={inputClass} type="datetime-local" value={draft.last_seen_at} onChange={(event) => update("last_seen_at", event.target.value)} /></Field>
        <Field label="体貌"><textarea aria-label="体貌" className={`${inputClass} min-h-28 resize-y`} value={draft.physical_description} maxLength={2_000} onChange={(event) => update("physical_description", event.target.value)} /></Field>
        <Field label="衣着"><textarea aria-label="衣着" className={`${inputClass} min-h-28 resize-y`} value={draft.clothing_description} maxLength={2_000} onChange={(event) => update("clothing_description", event.target.value)} /></Field>
        <Field label="健康注意"><textarea aria-label="健康注意" className={`${inputClass} min-h-28 resize-y`} value={draft.health_notes} maxLength={2_000} onChange={(event) => update("health_notes", event.target.value)} /></Field>
      </div>
      <div className="flex justify-end"><Button type="submit" variant="primary" isDisabled={isSaving}>{isSaving ? <Spinner size="sm" /> : "保存人物摘要"}</Button></div>
    </form>
  );
}

export function FamilyCaseProgressPage() {
  const { caseId } = useParams();
  const { token } = useAuth();
  const [detail, setDetail] = useState<CaseDetail | null>(null);
  const [progress, setProgress] = useState<CasePublicProgress | null>(null);
  const [resources, setResources] = useState(defaultResources);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [openSection, setOpenSection] = useState<"profile" | "clue" | null>(null);
  const [isSavingProfile, setIsSavingProfile] = useState(false);
  const [isSubmittingClue, setIsSubmittingClue] = useState(false);
  const [isSubmittingPlace, setIsSubmittingPlace] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [clue, setClue] = useState("");
  const [clueSourceType, setClueSourceType] = useState<PublicClueSourceType>("manual_report");
  const [clueOccurredAt, setClueOccurredAt] = useState("");
  const [clueLocation, setClueLocation] = useState("");
  const [clueLocationPrecision, setClueLocationPrecision] = useState<LocationPrecision | "">("");
  const [clueRawReference, setClueRawReference] = useState("");
  const [clueNextAction, setClueNextAction] = useState("");
  const [linkedAttachmentIds, setLinkedAttachmentIds] = useState<string[]>([]);
  const [place, setPlace] = useState<PlaceDraft>({ name: "", place_type: "frequent", address: "", longitude: "", latitude: "", visibility: "confirmed" });
  const [attachment, setAttachment] = useState<File | null>(null);
  const [previews, setPreviews] = useState<Record<string, string>>({});
  const [selectedPreview, setSelectedPreview] = useState<{ src: string; name: string } | null>(null);

  async function load() {
    if (!token || !caseId) return;
    setIsLoading(true);
    setError("");
    try {
      const [nextDetail, nextProgress, configuration] = await Promise.all([
        getCase(token, caseId),
        getCasePublicProgress(token, caseId),
        getCaseResourceConfiguration(token, caseId).catch(() => null),
      ]);
      setDetail(nextDetail);
      setProgress(nextProgress);
      if (configuration) setResources(configuration);
    } catch {
      setError("暂时无法读取这份家属案件，请稍后重试。");
    } finally {
      setIsLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // Fetch only when the session or route target changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [caseId, token]);

  const visibleAttachments = useMemo(
    () => detail?.attachments.filter((item) => item.is_own_submission) ?? [],
    [detail],
  );
  const visiblePlaces = useMemo(
    () => detail?.places.filter((item) => item.visibility !== "internal" && (item.review_status === "confirmed" || item.is_own_submission)) ?? [],
    [detail],
  );

  useEffect(() => {
    if (!token || !caseId || visibleAttachments.length === 0) {
      setPreviews({});
      return;
    }
    let active = true;
    const urls: string[] = [];
    void Promise.all(
      visibleAttachments.map(async (item) => {
        try {
          const blob = await downloadCaseAttachment(token, caseId, item.id);
          const url = URL.createObjectURL(blob);
          urls.push(url);
          return [item.id, url] as const;
        } catch {
          return null;
        }
      }),
    ).then((entries) => {
      if (active) setPreviews(Object.fromEntries(entries.filter((item): item is readonly [string, string] => item !== null)));
    });
    return () => {
      active = false;
      urls.forEach((url) => URL.revokeObjectURL(url));
    };
  }, [caseId, token, visibleAttachments]);

  async function saveProfile(payload: UpdateElderProfilePayload) {
    if (!token || !caseId) return;
    setIsSavingProfile(true);
    setError("");
    try {
      const updated = await updateElderProfile(token, caseId, payload);
      setDetail(updated);
      setNotice("人物摘要已保存。");
    } catch {
      setError("人物摘要暂时无法保存，请检查填写内容后重试。");
    } finally {
      setIsSavingProfile(false);
    }
  }

  async function submitClue(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!token || !caseId || !clue.trim()) return;
    if (clueLocationPrecision && !clueLocation.trim()) {
      setError("选择地点精度时，请同时填写地点。");
      return;
    }
    setIsSubmittingClue(true);
    setError("");
    try {
      await createClue(token, caseId, {
        source: "family",
        source_type: clueSourceType,
        content: clue.trim(),
        occurred_at: toIsoOrNull(clueOccurredAt),
        location_text: nullable(clueLocation),
        location_precision: clueLocationPrecision || null,
        raw_record_reference: nullable(clueRawReference),
        next_action: nullable(clueNextAction),
        attachment_ids: linkedAttachmentIds,
      });
      setClue(""); setClueOccurredAt(""); setClueLocation(""); setClueLocationPrecision(""); setClueRawReference(""); setClueNextAction(""); setLinkedAttachmentIds([]);
      setNotice("线索已提交，正在等待人工审核。");
      await load();
    } catch {
      setError("线索暂时没有提交成功，请检查网络后重试。");
    } finally {
      setIsSubmittingClue(false);
    }
  }

  async function submitPlace(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!token || !caseId || !place.name.trim() || !place.address.trim()) return;
    const coordinates = parseOptionalCoordinatePair(
      place.longitude,
      place.latitude,
    );
    if (!coordinates.ok) {
      setError(coordinates.message);
      return;
    }
    setIsSubmittingPlace(true);
    setError("");
    try {
      await createCasePlace(token, caseId, { ...place, name: place.name.trim(), address: place.address.trim(), longitude: coordinates.longitude, latitude: coordinates.latitude });
      setPlace({ name: "", place_type: resources.case_place_types[0] ?? "frequent", address: "", longitude: "", latitude: "", visibility: "confirmed" });
      setNotice("地点已提交，正在等待人工审核。");
      await load();
    } catch {
      setError("常去地点暂时没有保存成功，请稍后重试。");
    } finally {
      setIsSubmittingPlace(false);
    }
  }

  async function uploadPhoto(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!token || !caseId || !attachment) return;
    setIsUploading(true);
    setError("");
    try {
      await uploadCaseAttachment(token, caseId, attachment, resources.attachment_max_image_bytes);
      setAttachment(null);
      setNotice("图片已提交，正在等待人工审核。");
      await load();
    } catch {
      setError("图片未能提交。请使用 JPEG 或 PNG 图片，并检查文件大小。");
    } finally {
      setIsUploading(false);
    }
  }

  if (isLoading) return <main className="grid min-h-[calc(100vh-3.5rem)] place-items-center bg-[#f2f4f3]"><Spinner size="lg" aria-label="正在加载家属案件" /></main>;
  if (!detail || !progress) return <main className="min-h-[calc(100vh-3.5rem)] bg-[#f2f4f3] px-4 py-10"><div className="mx-auto max-w-xl border border-[#d8e3e0] bg-white p-6"><p role="alert" className="text-sm text-[#9e2b28]">{error || "找不到这份家属案件。"}</p><Link className="mt-5 inline-flex text-sm font-semibold text-[#0d5b56]" to="/family">返回家属端</Link></div></main>;

  const canAdd = detail.status === "active";
  const ownAttachments = detail.attachments.filter((item) => item.is_own_submission);
  return (
    <main className="min-h-[calc(100vh-3.5rem)] bg-[#f2f4f3] px-4 py-5 sm:px-6 lg:px-10 lg:py-8">
      <div className="mx-auto max-w-6xl">
        <header className="flex flex-col justify-between gap-4 border-b border-[#d8e3e0] pb-6 md:flex-row md:items-end">
          <div><Link to="/family" className="text-sm font-semibold text-[#0d5b56]">返回家属端</Link><div className="mt-3 flex flex-wrap items-center gap-3"><h1 className="text-2xl font-bold text-[#183330] sm:text-3xl">{detail.elder_profile.display_name || "待完善的老人资料"}</h1><span className="rounded-full border border-[#cfe0dc] px-2.5 py-1 text-xs font-semibold text-[#1e7d74]">案件 {detail.case_code}</span></div><p className="mt-2 text-sm text-[#667a78]">这里展示已审核的公开进展；您提交的资料、地点和线索会先进入人工审核。</p></div>
          <Button size="sm" variant="ghost" onPress={() => void load()}><RefreshCw size={15} aria-hidden="true" /> 刷新进展</Button>
        </header>
        {error && <p role="alert" className="mt-5 rounded-md border border-[#f1c7c5] bg-white px-4 py-3 text-sm text-[#9e2b28]">{error}</p>}
        {notice && <p role="status" className="mt-5 rounded-md border border-[#bcded6] bg-[#eef7f5] px-4 py-3 text-sm text-[#0d5b56]">{notice}</p>}

        <section className="mt-6 grid gap-5 lg:grid-cols-[minmax(0,1fr)_340px]">
          <div className="space-y-5">
            <section className="border border-[#d8e3e0] bg-white p-5 sm:p-6" aria-labelledby="public-progress-title"><div className="flex items-start justify-between gap-3"><div><p className="text-sm font-semibold text-[#1e7d74]">只看公开信息</p><h2 id="public-progress-title" className="mt-1 text-xl font-bold text-[#183330]">公开进展</h2></div><ShieldCheck className="text-[#0d5b56]" size={22} aria-hidden="true" /></div><div className="mt-5 space-y-4">{progress.confirmed_progress.length === 0 ? <p className="border border-dashed border-[#bdd1cc] px-4 py-5 text-sm text-[#667a78]">目前还没有新的公开进展。审核完成后会在这里更新。</p> : progress.confirmed_progress.map((item) => <div key={item.clue_id} className="flex gap-3 border-l-2 border-[#1e7d74] pl-4"><CheckCircle2 size={18} className="mt-0.5 shrink-0 text-[#1e7d74]" aria-hidden="true" /><div><p className="text-sm font-semibold text-[#183330]">已审核的进展更新</p><p className="mt-1 text-xs text-[#667a78]">更新时间：{dateLabel(item.updated_at)}</p></div></div>)}</div>{progress.safety_and_contact_reminders.length > 0 && <div className="mt-6 border-t border-[#edf1f0] pt-5"><h3 className="flex items-center gap-2 text-sm font-bold"><AlertTriangle size={16} className="text-[#f5a623]" aria-hidden="true" /> 温馨提醒</h3><ul className="mt-2 space-y-1 text-sm leading-6 text-[#667a78]">{progress.safety_and_contact_reminders.map((item) => <li key={item}>{item}</li>)}</ul></div>}</section>
            <section className="border border-[#d8e3e0] bg-white" aria-labelledby="profile-title"><button type="button" className="flex w-full items-center justify-between px-5 py-4 text-left sm:px-6" aria-expanded={openSection === "profile"} onClick={() => setOpenSection((current) => current === "profile" ? null : "profile")}><span><p className="text-sm font-semibold text-[#1e7d74]">家属可以补充</p><h2 id="profile-title" className="mt-1 text-xl font-bold text-[#183330]">补充或更正人物资料</h2></span><ChevronDown size={20} className={openSection === "profile" ? "rotate-180" : ""} /></button>{openSection === "profile" && <ElderProfileEditor detail={detail} isSaving={isSavingProfile} onSave={saveProfile} />}</section>
            {canAdd && <section className="border border-[#bcd1cc] bg-[#eef7f5]" aria-labelledby="new-clue-title"><button type="button" className="flex w-full items-center justify-between px-5 py-4 text-left sm:px-6" aria-expanded={openSection === "clue"} onClick={() => setOpenSection((current) => current === "clue" ? null : "clue")}><span className="flex items-center gap-3"><MessageCirclePlus size={21} className="text-[#0d5b56]" aria-hidden="true" /><span><h2 id="new-clue-title" className="text-lg font-bold text-[#183330]">提交一条新线索</h2><p className="mt-1 text-sm text-[#49615d]">只提交可核对的观察；提交后会进入人工审核。</p></span></span><ChevronDown size={20} className={openSection === "clue" ? "rotate-180" : ""} /></button>{openSection === "clue" && <form className="grid gap-5 border-t border-[#bcd1cc] p-5 sm:p-6 lg:grid-cols-[minmax(0,1fr)_280px]" onSubmit={(event) => void submitClue(event)}><div className="space-y-4"><Field label="线索内容" required><textarea aria-label="新线索内容" className={`${inputClass} min-h-36 resize-y`} value={clue} maxLength={4_000} onChange={(event) => setClue(event.target.value)} required /></Field><LocationConfirmationPicker onConfirm={(location) => { setClueLocation(location.address); setClueLocationPrecision(location.precision); }} onClear={() => { setClueLocation(""); setClueLocationPrecision(""); }} /></div><div className="space-y-4"><Field label="来源类型"><select aria-label="来源类型" className={inputClass} value={clueSourceType} onChange={(event) => setClueSourceType(event.target.value as PublicClueSourceType)}><option value="manual_report">人工上报</option><option value="field_report">现场反馈</option></select></Field><Field label="发生时间"><input aria-label="发生时间" type="datetime-local" className={inputClass} value={clueOccurredAt} onChange={(event) => setClueOccurredAt(event.target.value)} /></Field><Field label="地点"><input aria-label="地点" className={inputClass} value={clueLocation} maxLength={500} onChange={(event) => setClueLocation(event.target.value)} /></Field><Field label="地点精度"><select aria-label="地点精度" className={inputClass} value={clueLocationPrecision} onChange={(event) => setClueLocationPrecision(event.target.value as LocationPrecision | "")}><option value="">未提供</option><option value="exact">精确</option><option value="approximate">约略</option><option value="unknown">未知</option></select></Field><Field label="受控原始记录引用"><input aria-label="受控原始记录引用" className={inputClass} value={clueRawReference} maxLength={500} onChange={(event) => setClueRawReference(event.target.value)} /></Field><Field label="下一步动作"><input aria-label="下一步动作" className={inputClass} value={clueNextAction} maxLength={500} onChange={(event) => setClueNextAction(event.target.value)} /></Field>{ownAttachments.length > 0 && <Field label="关联本人附件"><select aria-label="关联本人附件" multiple className={`${inputClass} min-h-24`} value={linkedAttachmentIds} onChange={(event) => setLinkedAttachmentIds(Array.from(event.target.selectedOptions, (option) => option.value))}>{ownAttachments.map((item) => <option key={item.id} value={item.id}>{item.original_filename}</option>)}</select></Field>}<Button type="submit" variant="primary" fullWidth isDisabled={!clue.trim() || isSubmittingClue}>{isSubmittingClue ? <Spinner size="sm" /> : <Send size={16} aria-hidden="true" />} 提交线索</Button></div></form>}</section>}
          </div>

          <aside className="space-y-5">
            <section className="border border-[#d8e3e0] bg-white p-5" aria-labelledby="elder-summary-title"><h2 id="elder-summary-title" className="text-base font-bold text-[#183330]">老人资料摘要</h2><dl className="mt-4 space-y-3 text-sm"><div className="flex justify-between gap-3 border-b border-[#edf1f0] pb-2"><dt className="text-[#667a78]">年龄</dt><dd className="font-semibold">{detail.elder_profile.age ?? "未提供"}</dd></div><div className="flex justify-between gap-3 border-b border-[#edf1f0] pb-2"><dt className="text-[#667a78]">最后出现</dt><dd className="text-right font-semibold">{dateLabel(detail.elder_profile.last_seen_at)}</dd></div><div className="flex justify-between gap-3"><dt className="text-[#667a78]">地点</dt><dd className="max-w-[170px] text-right font-semibold">{detail.elder_profile.last_seen_location || "未提供"}</dd></div></dl></section>
            <section className="border border-[#d8e3e0] bg-white p-5" aria-labelledby="materials-title"><div className="flex items-center justify-between gap-2"><h2 id="materials-title" className="text-base font-bold text-[#183330]">资料管理</h2><FileImage size={18} className="text-[#1e7d74]" aria-hidden="true" /></div><p className="mt-2 text-xs leading-5 text-[#667a78]">仅显示您本人提交的私有图片。上传会由服务端重新编码并移除非必要的 EXIF/GPS 元数据。</p>{visibleAttachments.length === 0 ? <p className="mt-4 border border-dashed border-[#bcd1cc] px-3 py-4 text-xs text-[#667a78]">暂无可查看的图片</p> : <ul className="mt-4 grid gap-3">{visibleAttachments.map((item) => <li key={item.id} className="overflow-hidden border border-[#d8e3e0] bg-[#fbfcfc]"><button type="button" className="block w-full text-left" onClick={() => previews[item.id] && setSelectedPreview({ src: previews[item.id], name: item.original_filename })} disabled={!previews[item.id]}>{previews[item.id] ? <img src={previews[item.id]} alt={`${item.original_filename} 预览`} className="h-32 w-full object-cover" /> : <div className="grid h-24 place-items-center text-xs text-[#667a78]"><ImageIcon size={18} aria-hidden="true" /> 正在安全加载图片</div>}</button><div className="flex items-center justify-between gap-2 px-3 py-2 text-xs"><span className="truncate text-[#49615d]">{item.original_filename}</span><span className="shrink-0 text-[#667a78]">{attachmentStatus(item)}</span></div></li>)}</ul>}{canAdd && <form className="mt-4 grid gap-3" onSubmit={(event) => void uploadPhoto(event)}><input aria-label="选择图片" key={attachment ? `${attachment.name}-${attachment.lastModified}` : "no-file"} type="file" accept="image/jpeg,image/png,.jpg,.jpeg,.png" className="block w-full text-sm text-[#49615d]" onChange={(event) => setAttachment(event.target.files?.[0] ?? null)} /><Button type="submit" variant="secondary" isDisabled={!attachment || isUploading}>{isUploading ? <Spinner size="sm" /> : <Upload size={16} aria-hidden="true" />} 上传图片</Button><p className="text-xs text-[#667a78]">支持 JPEG、PNG，最大 {formatBytes(resources.attachment_max_image_bytes)}。</p></form>}</section>
          </aside>
        </section>

        {canAdd && <section className="mt-5 border border-[#d8e3e0] bg-white p-5 sm:p-6" aria-labelledby="places-title"><div className="flex flex-wrap items-center justify-between gap-3"><div><h2 id="places-title" className="flex items-center gap-2 text-xl font-bold text-[#183330]"><MapPin size={20} className="text-[#1e7d74]" aria-hidden="true" /> 常去地点</h2><p className="mt-1 text-sm text-[#667a78]">地点提交后由人工审核；可手动填写，也可用高德地图搜索或定位选点。</p></div><span className="text-xs font-semibold text-[#1e7d74]">提交后待人工审核</span></div><div className="mt-5 grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.15fr)]"><div><h3 className="text-sm font-bold text-[#49615d]">已提交地点</h3>{visiblePlaces.length === 0 ? <p className="mt-3 border border-dashed border-[#bcd1cc] px-4 py-5 text-sm text-[#667a78]">暂无可查看的补充地点</p> : <ul className="mt-3 divide-y divide-[#edf1f0] border border-[#d8e3e0] bg-[#fbfcfc]">{visiblePlaces.map((item) => <li key={item.id} className="px-4 py-3 text-sm"><div className="flex justify-between gap-3"><strong className="text-[#183330]">{item.name}</strong><span className="shrink-0 text-xs text-[#667a78]">{item.review_status === "confirmed" ? "已审核" : "待人工审核"}</span></div><p className="mt-1 text-[#667a78]">{item.address}</p></li>)}</ul>}</div><form className="grid gap-4" onSubmit={(event) => void submitPlace(event)}><div className="grid gap-4 sm:grid-cols-2"><Field label="地点名称" required><input aria-label="地点名称" className={inputClass} value={place.name} maxLength={120} onChange={(event) => setPlace({ ...place, name: event.target.value })} required /></Field><Field label="类型"><select aria-label="地点类型" className={inputClass} value={place.place_type} onChange={(event) => setPlace({ ...place, place_type: event.target.value })}>{resources.case_place_types.map((type) => <option key={type} value={type}>{placeTypeLabels[type] ?? type}</option>)}</select></Field></div><Field label="文字地址" required><input aria-label="文字地址" className={inputClass} value={place.address} maxLength={500} onChange={(event) => setPlace({ ...place, address: event.target.value })} required /></Field><LocationConfirmationPicker onConfirm={(location) => setPlace({ ...place, address: location.address, longitude: String(location.longitude), latitude: String(location.latitude) })} onClear={() => setPlace({ ...place, address: "", longitude: "", latitude: "" })} /><div className="grid gap-4 sm:grid-cols-3"><Field label="经度（可选）"><input aria-label="经度" className={inputClass} type="text" inputMode="decimal" value={place.longitude} onChange={(event) => setPlace({ ...place, longitude: event.target.value })} /></Field><Field label="纬度（可选）"><input aria-label="纬度" className={inputClass} type="text" inputMode="decimal" value={place.latitude} onChange={(event) => setPlace({ ...place, latitude: event.target.value })} /></Field><Field label="可见级别"><select aria-label="可见级别" className={inputClass} value={place.visibility} onChange={(event) => setPlace({ ...place, visibility: event.target.value as CreateCasePlacePayload["visibility"] })}><option value="confirmed">已确认范围</option><option value="public">公开范围</option><option value="internal">仅内部</option></select></Field></div><Button type="submit" variant="primary" isDisabled={isSubmittingPlace || !place.name.trim() || !place.address.trim()}>{isSubmittingPlace ? <Spinner size="sm" /> : <MapPin size={16} aria-hidden="true" />} 提交地点</Button></form></div></section>}
      </div>
      {selectedPreview && <div className="fixed inset-0 z-[100] grid place-items-center bg-black/70 p-4" role="dialog" aria-modal="true" aria-label={`${selectedPreview.name} 图片预览`}><div className="max-h-full max-w-4xl overflow-auto bg-white p-3"><div className="mb-3 flex items-center justify-between gap-4"><strong className="truncate text-sm text-[#183330]">{selectedPreview.name}</strong><Button type="button" size="sm" variant="ghost" isIconOnly aria-label="关闭图片预览" onPress={() => setSelectedPreview(null)}><X size={18} /></Button></div><img src={selectedPreview.src} alt={selectedPreview.name} className="max-h-[80vh] max-w-full object-contain" /></div></div>}
    </main>
  );
}
