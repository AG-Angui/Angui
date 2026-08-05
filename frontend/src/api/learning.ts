import { apiRequest } from "./client";

export interface LearningResource {
  id: string;
  title: string;
  summary: string;
  content: string;
  resource_type: "team_intro" | "manual" | "prevention" | "case_study";
  tags: string[];
  source_name: string;
  source_url: string | null;
  previous_version_id?: string | null;
  version: number;
  effective_at: string;
}

export interface LearningQuestionOption { id: string; text: string; }
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
export interface LearningAnswer { question_id: string; is_correct: boolean; explanation: string; source: LearningSource; }
export interface LearningSource { resource_id: string; title: string; version: number; }
export interface KnowledgeAnswer { answer: string; certainty: "source_backed" | "insufficient_sources"; sources: LearningSource[]; human_review_notice: string; }
export type LearningLifecycleState = "submitted" | "deidentified" | "reviewed" | "published" | "withdrawn" | "unmanaged";
export interface LearningContentReviewEvent { event_type: string; actor_user_id: string; reason: string; created_at: string; }
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
export interface ManagedLearningResource extends LearningResource { lifecycle: LearningContentLifecycle; }
export interface ManagedLearningQuestion extends LearningQuestion { lifecycle: LearningContentLifecycle; }
export interface CreateLearningResourceInput {
  title: string; summary: string; content: string; resource_type: LearningResource["resource_type"];
  tags: string[]; source_name: string; source_url: string | null; visibility: "public" | "authenticated" | "volunteer" | "learner";
  effective_at: string; permitted_use: "training" | "public_information"; submission_reason: string; previous_version_id?: string | null;
}
export interface CreateLearningQuestionInput {
  source_resource_id: string; prompt: string; question_type: LearningQuestion["question_type"];
  difficulty: LearningQuestion["difficulty"]; tags: string[]; options: LearningQuestionOption[];
  correct_option_id: string; explanation: string; visibility: "authenticated" | "volunteer" | "learner";
  effective_at: string; permitted_use: "training"; submission_reason: string; previous_version_id?: string | null;
}

export const listLearningResources = (token: string) => apiRequest<LearningResource[]>("/learning/resources", {}, token);
export const getPublicPreventionCard = () => apiRequest<LearningResource>("/learning/public/prevention-card");
export const listLearningQuestions = (token: string) => apiRequest<LearningQuestion[]>("/learning/questions", {}, token);
export const submitLearningAnswer = (token: string, questionId: string, selectedOptionId: string) => apiRequest<LearningAnswer>(`/learning/questions/${questionId}/answers`, { method: "POST", body: JSON.stringify({ selected_option_id: selectedOptionId }) }, token);
export const askKnowledge = (token: string, question: string) => apiRequest<KnowledgeAnswer>("/knowledge/ask", { method: "POST", body: JSON.stringify({ question }) }, token);
export const listManagedLearningResources = (token: string) => apiRequest<ManagedLearningResource[]>("/admin/learning/resources", {}, token);
export const listManagedLearningQuestions = (token: string) => apiRequest<ManagedLearningQuestion[]>("/admin/learning/questions", {}, token);
export const createManagedLearningResource = (token: string, input: CreateLearningResourceInput) => apiRequest<ManagedLearningResource>("/admin/learning/resources", { method: "POST", body: JSON.stringify(input) }, token);
export const createManagedLearningQuestion = (token: string, input: CreateLearningQuestionInput) => apiRequest<ManagedLearningQuestion>("/admin/learning/questions", { method: "POST", body: JSON.stringify(input) }, token);
export const transitionManagedLearningResource = (token: string, resourceId: string, action: "deidentify" | "review" | "publish" | "withdraw", reason: string) => apiRequest<ManagedLearningResource>(`/admin/learning/resources/${resourceId}/${action}`, { method: "POST", body: JSON.stringify({ reason }) }, token);
export const transitionManagedLearningQuestion = (token: string, questionId: string, action: "deidentify" | "review" | "publish" | "withdraw", reason: string) => apiRequest<ManagedLearningQuestion>(`/admin/learning/questions/${questionId}/${action}`, { method: "POST", body: JSON.stringify({ reason }) }, token);
