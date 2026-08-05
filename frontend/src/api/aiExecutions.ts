import { apiRequest } from "./client";

export type AiExecutionStage =
  | "queued"
  | "preparing"
  | "generating"
  | "validating"
  | "fallback"
  | "ready_for_review"
  | "failed";

export interface AiExecution {
  execution_id: string;
  workflow: "intake_profile_draft" | "intake_initial_review" | string;
  stage: AiExecutionStage;
  status: "running" | "completed" | "failed";
  failure_kind: string | null;
  result_status: string | null;
  fallback_used: boolean;
  last_event_id: number;
  created_at: string;
  updated_at: string;
}

export function getAiExecution(token: string, executionId: string) {
  return apiRequest<AiExecution>(
    `/ai/executions/${encodeURIComponent(executionId)}`,
    {},
    token,
  );
}
