import { apiRequest } from "./client";

export interface IntakeQuestion {
  field: string;
  prompt: string;
  required: boolean;
}

export interface IntakeSession {
  id: string;
  status: "collecting" | "ready_for_confirmation";
  missing_fields: string[];
  phase: "phase_one" | "phase_two";
  completed_phase_one_fields: string[];
  missing_phase_one_fields: string[];
  phase_transition_ready: boolean;
  next_question: IntakeQuestion | null;
  guidance_mode: "rule_based";
  privacy_notice: string;
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
  confirmation_status: "human_confirmed";
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
