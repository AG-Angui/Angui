import {
  Check,
  ChevronLeft,
  CircleHelp,
  FileText,
  HeartHandshake,
  LockKeyhole,
  MapPin,
  UserRound,
} from "lucide-react";
import { Link, useNavigate } from "react-router";
import { FamilyIntakeForm } from "./FamilyIntakeForm";

const intakeSteps = [
  { label: "基本信息", detail: "姓名、年龄与照片", icon: UserRound },
  { label: "身体情况", detail: "特征、衣着与照护", icon: HeartHandshake },
  { label: "最后出现", detail: "时间、地点与同行情况", icon: MapPin },
  { label: "习惯线索", detail: "常去地点与日常习惯", icon: CircleHelp },
  { label: "联系方式", detail: "便于人工核对", icon: FileText },
];

export function FamilyIntakePage() {
  const navigate = useNavigate();

  return (
    <main className="min-h-[calc(100vh-3.5rem)] bg-[#f2f4f3] text-[#183330]">
      <header className="border-b border-[#d8e3e0] bg-white px-4 py-4 sm:px-6 lg:px-10">
        <div className="mx-auto flex max-w-[1440px] items-center justify-between gap-3">
          <div className="min-w-0">
            <Link to="/family" className="inline-flex min-h-11 items-center gap-1 text-sm font-semibold text-[#0d5b56] hover:text-[#1e7d74]">
              <ChevronLeft size={18} aria-hidden="true" /> 返回家属端
            </Link>
            <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1">
              <h1 className="text-xl font-bold sm:text-2xl">填写求助信息</h1>
              <span className="text-sm text-[#667a78]">离开页面后，已填写的内容会保存在本次草稿中</span>
            </div>
          </div>
          <span className="hidden shrink-0 rounded-full border border-[#cfe0dc] px-3 py-1.5 text-sm font-semibold text-[#1e7d74] sm:inline">建案资料</span>
        </div>
      </header>

      <div className="mx-auto grid max-w-[1440px] gap-5 px-4 py-5 sm:px-6 lg:grid-cols-[225px_minmax(0,1fr)_280px] lg:px-10 lg:py-8">
        <nav className="hidden border border-[#d8e3e0] bg-white p-3 lg:block" aria-label="建案步骤">
          <p className="px-3 pb-3 text-sm font-semibold text-[#1e7d74]">建案步骤</p>
          <ol className="space-y-1">
            {intakeSteps.map(({ label, detail, icon: Icon }, index) => (
              <li key={label} className={index === 0 ? "border-l-2 border-[#0d5b56] bg-[#eef7f5]" : "border-l-2 border-transparent"}>
                <div className="flex gap-3 px-3 py-3">
                  <span className={index === 0 ? "grid size-7 shrink-0 place-items-center rounded-full bg-[#0d5b56] text-white" : "grid size-7 shrink-0 place-items-center rounded-full border border-[#cfe0dc] text-[#667a78]"}>
                    {index === 0 ? <Check size={15} aria-hidden="true" /> : <Icon size={15} aria-hidden="true" />}
                  </span>
                  <span className="min-w-0"><strong className="block text-sm text-[#183330]">{label}</strong><span className="mt-0.5 block text-xs leading-5 text-[#667a78]">{detail}</span></span>
                </div>
              </li>
            ))}
          </ol>
          <div className="mt-4 border-t border-[#edf1f0] px-3 pt-4 text-xs leading-5 text-[#667a78]">
            只需先提供您能确认的信息。不知道或不确定也可以继续。
          </div>
        </nav>

        <section className="min-w-0" aria-label="家属建案问询">
          <div className="mb-4 flex gap-1 overflow-x-auto lg:hidden" aria-label="建案进度">
            {intakeSteps.map(({ label }, index) => (
              <span key={label} className={index === 0 ? "shrink-0 border-b-2 border-[#0d5b56] px-3 py-2 text-sm font-semibold text-[#0d5b56]" : "shrink-0 border-b-2 border-transparent px-3 py-2 text-sm text-[#667a78]"}>{index + 1}. {label}</span>
            ))}
          </div>
          <FamilyIntakeForm
            onCancel={() => navigate("/family")}
            onConfirmed={async (caseId) => {
              navigate(`/family/cases/${caseId}`, { replace: true });
            }}
          />
          <div className="sticky bottom-0 z-20 mt-4 border border-[#d8e3e0] bg-white/95 px-4 py-3 text-sm text-[#49615d] backdrop-blur lg:hidden">
            每一步都可标记“不知道”；确认提交前不会创建正式案件。
          </div>
        </section>

        <aside className="hidden space-y-5 lg:block">
          <section className="border border-[#d8e3e0] bg-white p-5" aria-labelledby="family-profile-preview-title">
            <div className="flex items-center justify-between gap-2">
              <h2 id="family-profile-preview-title" className="text-base font-bold">老人画像预览</h2>
              <span className="text-xs font-semibold text-[#1e7d74]">会随填写更新</span>
            </div>
            <div className="mt-5 grid aspect-[4/3] place-items-center border border-dashed border-[#bdd1cc] bg-[#f7faf9] text-center">
              <UserRound size={32} className="text-[#8da6a1]" aria-hidden="true" />
              <p className="-mt-8 px-5 text-sm leading-6 text-[#667a78]">照片、姓名和最后出现信息会在这里汇总，提交前仍由您逐项确认。</p>
            </div>
            <dl className="mt-5 space-y-3 text-sm">
              <div className="flex justify-between gap-3 border-b border-[#edf1f0] pb-2"><dt className="text-[#667a78]">关键资料</dt><dd className="font-semibold text-[#49615d]">待填写</dd></div>
              <div className="flex justify-between gap-3 border-b border-[#edf1f0] pb-2"><dt className="text-[#667a78]">最后出现</dt><dd className="font-semibold text-[#49615d]">待填写</dd></div>
              <div className="flex justify-between gap-3"><dt className="text-[#667a78]">可用照片</dt><dd className="font-semibold text-[#49615d]">待上传</dd></div>
            </dl>
          </section>
          <section className="border-l-2 border-[#1e7d74] bg-[#eaf4f1] px-4 py-4 text-sm leading-6 text-[#49615d]">
            <div className="flex items-center gap-2 font-semibold text-[#0d5b56]"><LockKeyhole size={16} aria-hidden="true" /> 隐私说明</div>
            <p className="mb-0 mt-2">系统会提示可能需要补充的信息，但不会替您判断事实，也不会自动建立案件。</p>
          </section>
        </aside>
      </div>
    </main>
  );
}
