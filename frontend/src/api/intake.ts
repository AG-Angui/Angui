import { apiRequest } from './client'

export interface IntakeQuestion { field: string; prompt: string; required: boolean }

export interface IntakeSession {
  id: string
  status: 'collecting' | 'ready_for_confirmation'
  missing_fields: string[]
  phase: 'phase_one' | 'phase_two'
  missing_phase_one_fields: string[]
  phase_transition_ready: boolean
  next_question: IntakeQuestion | null
}

export interface IntakeAssessment {
  field_path: string
  conflict_type: string
  severity: 'info' | 'warning' | 'blocking'
  evidence_summary: string
  suggested_action: string
}

export interface IntakeDraft {
  status: 'draft'
  requires_human_confirmation: true
  profile: {
    physical_description: string | null
    clothing_description: string | null
    health_notes: string | null
    mobility_notes: string | null
    transportation_ability: string | null
    frequent_locations: string | null
    last_seen_information: string | null
    behavior_habits: string | null
    suspicious_motive: string | null
  }
  missing_fields: string[]
  assessments: IntakeAssessment[]
  confirmation_blocked_reasons: string[]
  direction_hypotheses: Array<{ description: string; uncertainty_notice: string; source_fields: string[] }>
}

export interface ConfirmedIntakeProfile {
  display_name: string
  age: number | null
  gender: string | null
  physical_description: string | null
  clothing_description: string | null
  health_notes: string | null
  last_seen_at: string | null
  last_seen_location: string
}

export interface ConfirmIntakeResponse { case_id: string; case_code: string; status: 'active'; confirmation_status: 'human_confirmed'; confirmed_at: string }

export function createIntakeSession(token: string): Promise<IntakeSession> {
  return apiRequest<IntakeSession>('/intake-sessions', { method: 'POST', body: JSON.stringify({}) }, token)
}

export function submitIntakeAnswer(token: string, sessionId: string, field: string, answer: string): Promise<IntakeSession & { assessments: IntakeAssessment[] }> {
  return apiRequest<IntakeSession & { assessments: IntakeAssessment[] }>(`/intake-sessions/${sessionId}/answers`, { method: 'POST', body: JSON.stringify({ field, answer }) }, token)
}

export function getIntakeDraft(token: string, sessionId: string): Promise<IntakeDraft> {
  return apiRequest<IntakeDraft>(`/intake-sessions/${sessionId}/profile-draft`, {}, token)
}

export function confirmIntakeSession(token: string, sessionId: string, profile: ConfirmedIntakeProfile): Promise<ConfirmIntakeResponse> {
  return apiRequest<ConfirmIntakeResponse>(`/intake-sessions/${sessionId}/confirm`, { method: 'POST', body: JSON.stringify({ human_confirmed: true, profile }) }, token)
}
