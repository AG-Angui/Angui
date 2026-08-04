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
  version: number;
}
export interface LearningAnswer { question_id: string; is_correct: boolean; explanation: string; source: LearningSource; }
export interface LearningSource { resource_id: string; title: string; version: number; }
export interface KnowledgeAnswer { answer: string; certainty: "source_backed" | "insufficient_sources"; sources: LearningSource[]; human_review_notice: string; }

export const listLearningResources = (token: string) => apiRequest<LearningResource[]>("/learning/resources", {}, token);
export const listLearningQuestions = (token: string) => apiRequest<LearningQuestion[]>("/learning/questions", {}, token);
export const submitLearningAnswer = (token: string, questionId: string, selectedOptionId: string) => apiRequest<LearningAnswer>(`/learning/questions/${questionId}/answers`, { method: "POST", body: JSON.stringify({ selected_option_id: selectedOptionId }) }, token);
export const askKnowledge = (token: string, question: string) => apiRequest<KnowledgeAnswer>("/knowledge/ask", { method: "POST", body: JSON.stringify({ question }) }, token);
