import { apiRequest } from './client'

export type CaseRole = 'family' | 'commander' | 'volunteer'
export type CaseStatus = 'active' | 'resolved' | 'closed'
export type ClueReviewStatus =
  | 'needs_verification'
  | 'confirmed'
  | 'rejected'
  | 'expired'
  | 'duplicate'
export type ClueStatus = 'pending_review' | ClueReviewStatus

export interface CaseListItem {
  id: string
  case_code: string
  status: CaseStatus
  access_role: CaseRole
  display_name: string
  last_seen_at: string | null
  last_seen_location: string | null
  created_at: string
  updated_at: string
}

export interface ElderProfile {
  id: string
  display_name: string
  age: number | null
  gender: string | null
  physical_description: string | null
  clothing_description: string | null
  health_notes: string | null
  last_seen_at: string | null
  last_seen_location: string | null
}

export interface Clue {
  id: string
  case_id: string
  status: ClueStatus
  source: string
  content: string
  occurred_at: string | null
  location_text: string | null
  created_at: string
  updated_at: string
  reviewed_at: string | null
  is_own_submission: boolean
}

export interface CaseDetail {
  id: string
  case_code: string
  status: CaseStatus
  access_role: CaseRole
  elder_profile: ElderProfile
  clues: Clue[]
  created_at: string
  updated_at: string
}

export interface CreateCasePayload {
  display_name: string
  age: number | null
  gender: string | null
  physical_description: string | null
  clothing_description: string | null
  health_notes: string | null
  last_seen_at: string | null
  last_seen_location: string | null
}

export interface CaseMember {
  user_id: string
  email: string
  display_name: string
  role: CaseRole
}

export interface CreateCluePayload {
  source: string
  content: string
  occurred_at: string | null
  location_text: string | null
}

export function listCases(token: string): Promise<CaseListItem[]> {
  return apiRequest<CaseListItem[]>('/cases', {}, token)
}

export function getCase(token: string, caseId: string): Promise<CaseDetail> {
  return apiRequest<CaseDetail>(`/cases/${caseId}`, {}, token)
}

export function createCase(token: string, payload: CreateCasePayload): Promise<CaseDetail> {
  return apiRequest<CaseDetail>(
    '/cases',
    { method: 'POST', body: JSON.stringify(payload) },
    token,
  )
}

export function createClue(
  token: string,
  caseId: string,
  payload: CreateCluePayload,
): Promise<Clue> {
  return apiRequest<Clue>(
    `/cases/${caseId}/clues`,
    { method: 'POST', body: JSON.stringify(payload) },
    token,
  )
}

export function reviewClue(
  token: string,
  clueId: string,
  status: ClueReviewStatus,
): Promise<Clue> {
  return apiRequest<Clue>(
    `/clues/${clueId}/review`,
    { method: 'PATCH', body: JSON.stringify({ status }) },
    token,
  )
}

export function updateCaseStatus(
  token: string,
  caseId: string,
  status: CaseStatus,
): Promise<CaseDetail> {
  return apiRequest<CaseDetail>(
    `/cases/${caseId}/status`,
    { method: 'PATCH', body: JSON.stringify({ status }) },
    token,
  )
}

export function addCaseMember(
  token: string,
  caseId: string,
  email: string,
  role: CaseRole,
): Promise<CaseMember> {
  return apiRequest<CaseMember>(
    `/cases/${caseId}/members`,
    { method: 'POST', body: JSON.stringify({ email, role }) },
    token,
  )
}
