import { ApiClientError, apiRequest } from './client'
import type { AccountType, GlobalCapability } from './auth'

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
  places: CasePlace[]
  attachments: CaseAttachment[]
  created_at: string
  updated_at: string
}

export type PlaceType = string
export type PlaceVisibility = 'public' | 'confirmed' | 'internal'

export interface CasePlace {
  id: string
  case_id: string
  name: string
  place_type: PlaceType
  address: string
  longitude: number | null
  latitude: number | null
  source: CaseRole
  visibility: PlaceVisibility
  review_status: 'pending_review' | 'confirmed' | 'rejected'
  created_at: string
  updated_at: string
  is_own_submission: boolean
}

export interface CaseAttachment {
  id: string
  case_id: string
  original_filename: string
  content_type: 'image/jpeg' | 'image/png'
  byte_size: number
  source: string
  review_status: 'pending_review' | 'confirmed' | 'rejected'
  created_at: string
  updated_at: string
  is_own_submission: boolean
}

export interface CreateCasePlacePayload {
  name: string
  place_type: PlaceType
  address: string
  longitude: number | null
  latitude: number | null
  visibility: PlaceVisibility
}

export interface CaseResourceConfiguration {
  attachment_max_image_bytes: number
  attachment_max_per_case: number
  case_place_types: string[]
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
  account_type: AccountType
  global_capabilities: GlobalCapability[]
  case_role: CaseRole
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

export function getCaseResourceConfiguration(
  token: string,
  caseId: string,
): Promise<CaseResourceConfiguration> {
  return apiRequest<CaseResourceConfiguration>(`/cases/${caseId}/resource-configuration`, {}, token)
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

export function createCasePlace(token: string, caseId: string, payload: CreateCasePlacePayload): Promise<CasePlace> {
  return apiRequest<CasePlace>(`/cases/${caseId}/places`, { method: 'POST', body: JSON.stringify(payload) }, token)
}

export function uploadCaseAttachment(
  token: string,
  caseId: string,
  file: File,
  maximumBytes: number,
): Promise<CaseAttachment> {
  if (!['image/jpeg', 'image/png'].includes(file.type)) {
    throw new ApiClientError(400, 'validation_error', '仅可上传 JPEG 或 PNG 图片。')
  }
  if (file.size > maximumBytes) {
    throw new ApiClientError(400, 'validation_error', `图片不能超过 ${maximumBytes} 字节。`)
  }
  const body = new FormData()
  body.append('file', file, file.name)
  return apiRequest<CaseAttachment>(`/cases/${caseId}/attachments`, { method: 'POST', body }, token)
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
  caseRole: CaseRole,
): Promise<CaseMember> {
  return apiRequest<CaseMember>(
    `/cases/${caseId}/members`,
    { method: 'POST', body: JSON.stringify({ email, case_role: caseRole }) },
    token,
  )
}
