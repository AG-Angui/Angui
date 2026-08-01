import { apiRequest } from "./client";

export interface IntakeQuestion {
  field: string;
  prompt: string;
  required: boolean;
}

export interface IntakeSession {
  id: string;
  status:
    | "collecting"
    | "ready_for_confirmation"
    | "awaiting_family_review"
    | "ready_for_second_confirmation";
  missing_fields: string[];
  phase: "phase_one" | "phase_two";
  completed_phase_one_fields: string[];
  missing_phase_one_fields: string[];
  phase_transition_ready: boolean;
  next_question: IntakeQuestion | null;
  guidance_mode: "rule_based" | "ai_assisted";
  ai_initial_review_status: string;
  privacy_notice: string;
}

export interface IntakeAiFollowUp {
  field: string;
  prompt: string;
  purpose: string;
  missing_fields: string[];
  skippable: boolean;
}

export interface IntakeAiFollowUpResponse {
  question: IntakeAiFollowUp | null;
  degradation_status: string;
  generated_at: string;
}

export type IntakeSessionUpdate = Omit<IntakeSession, "id">;

export interface IntakeCandidateField {
  field: string;
  value: string;
  source: "family_provided" | "ai_extracted";
  status: "draft";
  generated_at: string;
  model: string | null;
  template_version: string | null;
  source_text: string;
  confidence: number | null;
}

export interface IntakeRouteEstimate {
  distance_meters: number;
  available_seconds: number;
  minimum_seconds: number | null;
  basis: string;
  degraded: boolean;
}

export interface IntakeAssessment {
  field_path: string;
  conflict_type: string;
  severity: "info" | "warning" | "blocking";
  evidence_summary: string;
  suggested_action: string;
  route_estimate: IntakeRouteEstimate | null;
}

export interface SubmitIntakeAnswerResponse extends IntakeSessionUpdate {
  session_id: string;
  raw_answer: string;
  candidate_fields: IntakeCandidateField[];
  assessments: IntakeAssessment[];
}

export interface IntakeProfileDraftFieldMetadata {
  field: string;
  source_field: string;
  source: "family_provided" | "ai_extracted";
  status: "draft";
  generated_at: string;
}

export interface IntakeDraftProfile {
  physical_description: string | null;
  clothing_description: string | null;
  health_notes: string | null;
  mobility_notes: string | null;
  transportation_ability: string | null;
  frequent_locations: string | null;
  last_seen_information: string | null;
  behavior_habits: string | null;
  suspicious_motive: string | null;
}

export interface IntakeDirectionHypothesis {
  status: "hypothesis";
  description: string;
  uncertainty_notice: string;
  source_fields: string[];
  generated_at: string;
}

export interface IntakeDraft {
  id: string;
  status: "draft";
  source_scope: string;
  generated_at: string;
  requires_human_confirmation: true;
  profile: IntakeDraftProfile;
  field_metadata: IntakeProfileDraftFieldMetadata[];
  missing_fields: string[];
  assessments: IntakeAssessment[];
  confirmation_blocked_reasons: string[];
  direction_hypotheses: IntakeDirectionHypothesis[];
  provider_model: string | null;
  template_version: string;
  degradation_status: string;
  version: number;
}

export interface IntakeDraftDiff {
  from_version: number;
  to_version: number;
  changed_fields: string[];
}

export interface ConfirmedIntakeProfile {
  display_name: string;
  age: number | null;
  gender: string | null;
  physical_description: string | null;
  clothing_description: string | null;
  health_notes: string | null;
  last_seen_at: string | null;
  last_seen_location: string;
}

export interface ConfirmIntakeResponse {
  case_id: string;
  case_code: string;
  status: "active" | "resolved" | "closed";
  confirmation_status:
    "human_confirmed" | "human_confirmed_after_ai_initial_review";
  confirmed_at: string;
}

export function createIntakeSession(token: string): Promise<IntakeSession> {
  return apiRequest<IntakeSession>(
    "/intake-sessions",
    { method: "POST", body: JSON.stringify({}) },
    token,
  );
}

export function submitIntakeAnswer(
  token: string,
  sessionId: string,
  payload: { field: string; answer: string; replace?: boolean },
): Promise<SubmitIntakeAnswerResponse> {
  return apiRequest<SubmitIntakeAnswerResponse>(
    `/intake-sessions/${sessionId}/answers`,
    { method: "POST", body: JSON.stringify(payload) },
    token,
  );
}

export function getIntakeDraft(
  token: string,
  sessionId: string,
): Promise<IntakeDraft> {
  return apiRequest<IntakeDraft>(
    `/intake-sessions/${sessionId}/profile-draft`,
    {},
    token,
  );
}

export function generateIntakeDraft(
  token: string,
  sessionId: string,
): Promise<IntakeDraft> {
  return apiRequest(
    `/intake-sessions/${sessionId}/profile-draft/generate`,
    { method: "POST" },
    token,
  );
}
export function listIntakeDraftVersions(
  token: string,
  sessionId: string,
): Promise<{ items: IntakeDraft[] }> {
  return apiRequest(
    `/intake-sessions/${sessionId}/profile-draft/versions`,
    {},
    token,
  );
}
export function diffIntakeDraftVersions(
  token: string,
  sessionId: string,
  fromId: string,
  toId: string,
): Promise<IntakeDraftDiff> {
  return apiRequest(
    `/intake-sessions/${sessionId}/profile-draft/${fromId}/diff/${toId}`,
    {},
    token,
  );
}
export function reviewIntakeDraft(
  token: string,
  sessionId: string,
  draftId: string,
  action: "confirm" | "reject",
  reason: string,
): Promise<IntakeDraft> {
  return apiRequest(
    `/intake-sessions/${sessionId}/profile-draft/${draftId}/review`,
    { method: "PATCH", body: JSON.stringify({ action, reason }) },
    token,
  );
}
export function restoreIntakeDraft(
  token: string,
  sessionId: string,
  draftId: string,
  reason: string,
): Promise<IntakeDraft> {
  return apiRequest(
    `/intake-sessions/${sessionId}/profile-draft/restore`,
    { method: "POST", body: JSON.stringify({ draft_id: draftId, reason }) },
    token,
  );
}

export interface IntakeAiInitialReviewIssue {
  id: string;
  field: string;
  severity: "needs_confirmation" | "warning";
  evidence_summary: string;
  clarification_question: string;
  source_fields: string[];
}

export interface IntakeAiInitialReviewResponse {
  session_id: string;
  status: IntakeSession["status"];
  degradation_status: "available" | "rule_based_fallback" | "not_started";
  issues: IntakeAiInitialReviewIssue[];
  blocking_assessments: IntakeAssessment[];
  generated_at: string;
  requires_family_acknowledgement: boolean;
  ready_for_second_confirmation: boolean;
}

export interface IntakeAnswerRevision {
  id: string;
  field: string;
  answer: string;
  revision_kind: string;
  created_at: string;
}

export function getIntakeAiFollowUp(
  token: string,
  sessionId: string,
): Promise<IntakeAiFollowUpResponse> {
  return apiRequest<IntakeAiFollowUpResponse>(
    `/intake-sessions/${sessionId}/ai-follow-up`,
    {},
    token,
  );
}

export function getIntakeAiInitialReview(
  token: string,
  sessionId: string,
): Promise<IntakeAiInitialReviewResponse> {
  return apiRequest<IntakeAiInitialReviewResponse>(
    `/intake-sessions/${sessionId}/ai-initial-review`,
    {},
    token,
  );
}

export function startIntakeAiInitialReview(
  token: string,
  sessionId: string,
  profile: ConfirmedIntakeProfile,
): Promise<IntakeAiInitialReviewResponse> {
  return apiRequest<IntakeAiInitialReviewResponse>(
    `/intake-sessions/${sessionId}/ai-initial-review`,
    { method: "POST", body: JSON.stringify({ profile }) },
    token,
  );
}

export function acknowledgeIntakeAiInitialReview(
  token: string,
  sessionId: string,
  confirmedIssueIds: string[],
): Promise<IntakeAiInitialReviewResponse> {
  return apiRequest<IntakeAiInitialReviewResponse>(
    `/intake-sessions/${sessionId}/ai-initial-review/acknowledge`,
    {
      method: "POST",
      body: JSON.stringify({
        human_confirmed: true,
        confirmed_issue_ids: confirmedIssueIds,
      }),
    },
    token,
  );
}

export function listIntakeAnswerRevisions(
  token: string,
  sessionId: string,
): Promise<IntakeAnswerRevision[]> {
  return apiRequest(
    `/intake-sessions/${sessionId}/answer-revisions`,
    {},
    token,
  );
}

export function restoreIntakeAnswerRevision(
  token: string,
  sessionId: string,
  field: string,
  revisionId: string,
): Promise<SubmitIntakeAnswerResponse> {
  return apiRequest(
    `/intake-sessions/${sessionId}/answers/${encodeURIComponent(field)}/restore`,
    { method: "POST", body: JSON.stringify({ revision_id: revisionId }) },
    token,
  );
}

export function confirmIntakeSession(
  token: string,
  sessionId: string,
  profile: ConfirmedIntakeProfile,
): Promise<ConfirmIntakeResponse> {
  return apiRequest<ConfirmIntakeResponse>(
    `/intake-sessions/${sessionId}/confirm`,
    {
      method: "POST",
      body: JSON.stringify({ human_confirmed: true, profile }),
    },
    token,
  );
}
