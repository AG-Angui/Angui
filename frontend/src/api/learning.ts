import { apiRequest } from "./client";

export interface LearningResource {
  id: string;
  title: string;
  summary: string;
  content: string;
  resource_type: "team_intro" | "manual" | "prevention" | "case_study";
  tags: string[];
  category?: LearningCategory | null;
  source_name: string;
  source_url: string | null;
  previous_version_id?: string | null;
  version: number;
  effective_at: string;
}
export interface LearningCategory {
  id: string;
  name: string;
  status: "pending" | "enabled" | "rejected" | "disabled" | "assigned";
}
export interface ManagedLearningCategory extends LearningCategory {
  submitted_by_user_id: string;
  reviewed_by_user_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface LearningQuestionOption {
  id: string;
  text: string;
}
export interface LearningQuestion {
  id: string;
  prompt: string;
  question_type: "single_choice" | "true_false" | "scenario";
  difficulty: "basic" | "intermediate" | "advanced";
  tags: string[];
  options: LearningQuestionOption[];
  source_resource_id: string;
  previous_version_id?: string | null;
  version: number;
}
export interface LearningAnswer {
  question_id: string;
  is_correct: boolean;
  explanation: string;
  source: LearningSource;
}
export interface KnowledgeImage {
  id: string;
  storage_path: string;
  mime_type: string;
  width: number | null;
  height: number | null;
  metadata: Record<string, unknown>;
}
export interface LearningSource {
  knowledge_item_id: string;
  title: string;
  version: number;
  score: number;
  images: KnowledgeImage[];
}
export interface KnowledgeAnswer {
  answer: string;
  certainty: "source_backed" | "rule_based" | "insufficient_sources";
  sources: LearningSource[];
  human_review_notice: string;
}
export type LearningLifecycleState =
  | "submitted"
  | "deidentified"
  | "reviewed"
  | "published"
  | "withdrawn"
  | "unmanaged";
export interface LearningContentReviewEvent {
  event_type: string;
  actor_user_id: string;
  reason: string;
  created_at: string;
}
export interface LearningContentLifecycle {
  submitted_by_user_id: string;
  deidentified_by_user_id: string | null;
  reviewed_by_user_id: string | null;
  published_by_user_id: string | null;
  withdrawn_by_user_id: string | null;
  state: LearningLifecycleState;
  permitted_use: "training" | "public_information";
  events: LearningContentReviewEvent[];
}
export interface ManagedLearningResource extends LearningResource {
  lifecycle: LearningContentLifecycle;
}
export interface ManagedLearningQuestion extends LearningQuestion {
  lifecycle: LearningContentLifecycle;
}
export interface CreateLearningResourceInput {
  title: string;
  summary: string;
  content: string;
  resource_type: LearningResource["resource_type"];
  tags: string[];
  category_id?: string | null;
  source_name: string;
  source_url: string | null;
  visibility: "public" | "authenticated" | "volunteer" | "learner";
  effective_at: string;
  permitted_use: "training" | "public_information";
  submission_reason: string;
  previous_version_id?: string | null;
}
export interface CreateLearningQuestionInput {
  source_resource_id: string;
  prompt: string;
  question_type: LearningQuestion["question_type"];
  difficulty: LearningQuestion["difficulty"];
  tags: string[];
  options: LearningQuestionOption[];
  correct_option_id: string;
  explanation: string;
  visibility: "authenticated" | "volunteer" | "learner";
  effective_at: string;
  permitted_use: "training";
  submission_reason: string;
  previous_version_id?: string | null;
}

export const listLearningResources = (
  token: string,
  filters: { category_id?: string; tag?: string } = {},
) => {
  const query = new URLSearchParams(
    Object.entries(filters).filter(([, value]) => Boolean(value)) as [string, string][],
  );
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<LearningResource[]>(`/learning/resources${suffix}`, {}, token);
};
export const listLearningCategories = (token: string) =>
  apiRequest<LearningCategory[]>("/learning/categories", {}, token);
export const submitLearningCategoryProposal = (
  token: string,
  name: string,
  submission_reason: string,
) => apiRequest<LearningCategory>(
  "/learning/categories/proposals",
  { method: "POST", body: JSON.stringify({ name, submission_reason }) },
  token,
);
export const submitLearningResourceDraft = (
  token: string,
  input: CreateLearningResourceInput,
) => apiRequest<ManagedLearningResource>(
  "/learning/resources/drafts",
  { method: "POST", body: JSON.stringify(input) },
  token,
);
export const getPublicPreventionCard = () =>
  apiRequest<LearningResource>("/learning/public/prevention-card");
export const listLearningQuestions = (token: string) =>
  apiRequest<LearningQuestion[]>("/learning/questions", {}, token);
export const submitLearningAnswer = (
  token: string,
  questionId: string,
  selectedOptionId: string,
) =>
  apiRequest<LearningAnswer>(
    `/learning/questions/${questionId}/answers`,
    {
      method: "POST",
      body: JSON.stringify({ selected_option_id: selectedOptionId }),
    },
    token,
  );
export const askKnowledge = (token: string, question: string) =>
  apiRequest<KnowledgeAnswer>(
    "/knowledge-bases/learning-materials/chat",
    { method: "POST", body: JSON.stringify({ query: question, limit: 5 }) },
    token,
  );
export const listManagedLearningResources = (token: string) =>
  apiRequest<ManagedLearningResource[]>("/admin/learning/resources", {}, token);
export const listManagedLearningCategories = (token: string) =>
  apiRequest<ManagedLearningCategory[]>("/admin/learning/categories", {}, token);
export const transitionManagedLearningCategory = (
  token: string,
  categoryId: string,
  action: "enable" | "reject" | "disable",
  reason: string,
) => apiRequest<LearningCategory>(
  `/admin/learning/categories/${categoryId}/${action}`,
  { method: "POST", body: JSON.stringify({ reason }) },
  token,
);
export const listManagedLearningQuestions = (token: string) =>
  apiRequest<ManagedLearningQuestion[]>("/admin/learning/questions", {}, token);
export const createManagedLearningResource = (
  token: string,
  input: CreateLearningResourceInput,
) =>
  apiRequest<ManagedLearningResource>(
    "/admin/learning/resources",
    { method: "POST", body: JSON.stringify(input) },
    token,
  );
export const createManagedLearningQuestion = (
  token: string,
  input: CreateLearningQuestionInput,
) =>
  apiRequest<ManagedLearningQuestion>(
    "/admin/learning/questions",
    { method: "POST", body: JSON.stringify(input) },
    token,
  );
export const transitionManagedLearningResource = (
  token: string,
  resourceId: string,
  action: "deidentify" | "review" | "publish" | "withdraw",
  reason: string,
) =>
  apiRequest<ManagedLearningResource>(
    `/admin/learning/resources/${resourceId}/${action}`,
    { method: "POST", body: JSON.stringify({ reason }) },
    token,
);

export interface KnowledgeBase { id: string; name: string; description: string; visibility: string; status: string; created_at: string; updated_at: string; }
export interface KnowledgeImportRow { id: string; row_number: number; status: string; error_message: string | null; normalized_data: Record<string, unknown>; knowledge_item_id: string | null; }
export interface KnowledgeImportBatch { id: string; knowledge_base_id: string; file_name: string; status: string; total_rows: number; valid_rows: number; invalid_rows: number; rows: KnowledgeImportRow[]; confirmed_at: string | null; }
export interface KnowledgeItem { knowledge_item_id: string; title: string; content: string; score: number; knowledge_base_id: string; version: number; source_name: string; source_url: string | null; images: KnowledgeImage[]; }
export const listKnowledgeBases = (token: string) => apiRequest<KnowledgeBase[]>("/admin/knowledge-bases", {}, token);
export const createKnowledgeBase = (token: string, input: { name: string; description: string; visibility: string }) => apiRequest<KnowledgeBase>("/admin/knowledge-bases", { method: "POST", body: JSON.stringify(input) }, token);
export const previewKnowledgeImport = (token: string, baseId: string, file: File) => { const form = new FormData(); form.append("file", file); return apiRequest<KnowledgeImportBatch>(`/admin/knowledge-bases/${baseId}/imports/preview`, { method: "POST", body: form }, token); };
export const getKnowledgeImport = (token: string, batchId: string) => apiRequest<KnowledgeImportBatch>(`/admin/knowledge-imports/${batchId}`, {}, token);
export const confirmKnowledgeImport = (token: string, batchId: string) => apiRequest<KnowledgeImportBatch>(`/admin/knowledge-imports/${batchId}/confirm`, { method: "POST" }, token);
export const cancelKnowledgeImport = (token: string, batchId: string) => apiRequest<KnowledgeImportBatch>(`/admin/knowledge-imports/${batchId}/cancel`, { method: "POST" }, token);
export const listKnowledgeItems = (token: string, baseId: string) => apiRequest<KnowledgeItem[]>(`/admin/knowledge-bases/${baseId}/items`, {}, token);
export const createKnowledgeItem = (token: string, baseId: string, input: { title: string; summary: string; content: string; category: string; category_id: string | null; keywords: string[]; source_name: string; source_url: string | null; visibility: string; images: { storage_path: string; mime_type: string; width: number | null; height: number | null; metadata: Record<string, unknown> }[] }) => apiRequest<KnowledgeItem>(`/admin/knowledge-bases/${baseId}/items`, { method: "POST", body: JSON.stringify(input) }, token);
export const transitionKnowledgeItem = (token: string, itemId: string, action: "review" | "publish" | "withdraw") => apiRequest<KnowledgeItem>(`/admin/knowledge-items/${itemId}/${action}`, { method: "POST" }, token);
export const transitionManagedLearningQuestion = (
  token: string,
  questionId: string,
  action: "deidentify" | "review" | "publish" | "withdraw",
  reason: string,
) =>
  apiRequest<ManagedLearningQuestion>(
    `/admin/learning/questions/${questionId}/${action}`,
    { method: "POST", body: JSON.stringify({ reason }) },
    token,
  );
