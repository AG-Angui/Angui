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
  | 'conflicting'
  | 'insufficient_information'
export type ClueStatus = 'pending_review' | ClueReviewStatus
export type ClueSourceType = 'manual_report' | 'field_report' | 'chat_draft' | 'ai_draft'
export type PublicClueSourceType = 'manual_report' | 'field_report'
export type LocationPrecision = 'exact' | 'approximate' | 'unknown'

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
  source_type: ClueSourceType
  content: string
  raw_record_reference: string | null
  occurred_at: string | null
  reported_at: string
  confirmed_at: string | null
  location_text: string | null
  location_precision: LocationPrecision | null
  next_action: string | null
  linked_task_reference: string | null
  related_clue_id: string | null
  relationship_type: 'duplicate_of' | 'conflicts_with' | null
  review_reason: string | null
  attachment_ids: string[]
  created_at: string
  updated_at: string
  reviewed_at: string | null
  is_own_submission: boolean
}

export interface ClueTimelineQuery {
  page?: number
  page_size?: number
  status?: ClueStatus
  source_type?: ClueSourceType
  sort?: 'created_at' | 'occurred_at'
  order?: 'asc' | 'desc'
}

export interface ClueTimelinePage {
  items: Clue[]
  page: number
  page_size: number
  total: number
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

export interface UpdateElderProfilePayload {
  display_name?: string
  age?: number
  gender?: string
  physical_description?: string
  clothing_description?: string
  health_notes?: string
  last_seen_at?: string
  last_seen_location?: string
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
  source_type?: PublicClueSourceType
  raw_record_reference?: string | null
  location_precision?: LocationPrecision | null
  next_action?: string | null
  linked_task_reference?: string | null
  attachment_ids?: string[]
}

export interface ReviewCluePayload {
  status: ClueReviewStatus
  reason: string
  related_clue_id?: string | null
  relationship_type?: 'duplicate_of' | 'conflicts_with' | null
  next_action?: string | null
  linked_task_reference?: string | null
}

export interface PublicProgressItem { clue_id: string; progress_type: 'confirmed_update' | 'family_follow_up'; review_status: string; updated_at: string }
export interface CasePublicProgress { case_id: string; status: CaseStatus; generated_at: string; confirmed_progress: PublicProgressItem[]; requested_family_information: PublicProgressItem[]; safety_and_contact_reminders: string[] }
export interface ClueDraft { id: string; case_id: string; status: 'draft' | 'pending_review'; content: string; source_type: PublicClueSourceType; raw_record_reference: string | null; occurred_at: string | null; location_text: string | null; uncertainty_notice: string; template_version: string; provider_model: string | null; degradation_status: string }
export interface SummaryDraft { id: string; case_id: string; status: 'draft' | 'pending_review' | 'published' | 'rejected' | 'withdrawn' | 'superseded'; content: string; source_scope: string[]; template_version: string; provider_model: string | null; generated_at: string; reviewed_at: string | null; review_reason: string | null; created_at: string; updated_at: string; publication_eligible: boolean }
export interface CasePoi { id: string; name: string; category: string; address: string | null; longitude: number | null; latitude: number | null }
export interface CasePois { items: CasePoi[]; source: string; degradation_status: string; fallback_message: string | null }
export type CaseMapObjectType = 'last_seen' | 'place' | 'clue' | 'task'
export type MapLocationPrecision = 'exact' | 'approximate' | 'unknown'
export interface CaseMapItem {
  id: string
  object_type: CaseMapObjectType
  display_name: string | null
  longitude: number | null
  latitude: number | null
  location_text: string | null
  location_precision: MapLocationPrecision
  source: string
  occurred_at: string | null
  reported_at: string | null
  review_status: string
  related_task_id: string | null
  updated_at: string
}
export interface CaseMapView { items: CaseMapItem[] }
export type TaskStatus = 'assigned' | 'accepted' | 'active' | 'blocked' | 'completed' | 'cancelled'
export interface CaseTask { id: string; case_id: string; source_clue_id: string | null; title: string; objective: string; area_text: string; latitude: number | null; longitude: number | null; due_at: string; background: string | null; risk_level: string; risk_notes: string; safety_briefing: string; expected_feedback: string; status: TaskStatus; result_summary: string | null; assigned_volunteer_user_id: string | null; assigned_at: string | null; created_at: string; updated_at: string }
export interface TaskListPage { items: CaseTask[]; page: number; page_size: number; total: number }
export interface CreateTaskPayload { source_clue_id: string; volunteer_user_id: string; title: string; objective: string; area_text: string; latitude: number | null; longitude: number | null; due_at: string; background: string; risk_level: 'low' | 'medium' | 'high'; risk_notes: string; safety_briefing: string; expected_feedback: string }

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

export function getCasePublicProgress(token: string, caseId: string): Promise<CasePublicProgress> {
  return apiRequest<CasePublicProgress>(`/cases/${caseId}/public-progress`, {}, token)
}

export function getCaseMapView(token: string, caseId: string): Promise<CaseMapView> {
  return apiRequest<CaseMapView>(`/cases/${caseId}/map-view`, {}, token)
}

export function listCaseTasks(token: string, caseId: string): Promise<TaskListPage> {
  return apiRequest<TaskListPage>(`/cases/${caseId}/tasks`, {}, token)
}

export function listCaseMembers(token: string, caseId: string): Promise<CaseMember[]> {
  return apiRequest<CaseMember[]>(`/cases/${caseId}/members`, {}, token)
}

export function createCaseTask(token: string, caseId: string, payload: CreateTaskPayload): Promise<CaseTask> {
  return apiRequest<CaseTask>(`/cases/${caseId}/tasks`, { method: 'POST', body: JSON.stringify(payload) }, token)
}

export function updateTaskStatus(token: string, taskId: string, status: TaskStatus): Promise<CaseTask> {
  return apiRequest<CaseTask>(`/tasks/${taskId}/status`, { method: 'PATCH', body: JSON.stringify({ status }) }, token)
}

export function createClueDraft(token: string, caseId: string, payload: { text: string; source_type?: PublicClueSourceType; raw_record_reference?: string }): Promise<ClueDraft[]> {
  return apiRequest<ClueDraft[]>(`/cases/${caseId}/clue-drafts`, { method: 'POST', body: JSON.stringify(payload) }, token)
}

export function listCasePois(token: string, caseId: string, category = 'hospital'): Promise<CasePois> {
  return apiRequest<CasePois>(`/cases/${caseId}/pois?category=${encodeURIComponent(category)}`, {}, token)
}

export function createSummaryDraft(token: string, caseId: string, content?: string): Promise<SummaryDraft> {
  return apiRequest<SummaryDraft>(`/cases/${caseId}/summary-drafts`, { method: 'POST', body: JSON.stringify(content === undefined ? {} : { content }) }, token)
}

export function reviewSummaryDraft(token: string, caseId: string, draftId: string, payload: { action: 'submit' | 'publish' | 'reject' | 'withdraw'; reason: string }): Promise<SummaryDraft> {
  return apiRequest<SummaryDraft>(`/cases/${caseId}/summary-drafts/${draftId}/review`, { method: 'PATCH', body: JSON.stringify(payload) }, token)
}

export function listCaseClues(
  token: string,
  caseId: string,
  query: ClueTimelineQuery = {},
): Promise<ClueTimelinePage> {
  const parameters = new URLSearchParams()
  for (const [key, value] of Object.entries(query)) {
    if (value !== undefined) parameters.set(key, String(value))
  }
  const suffix = parameters.size > 0 ? `?${parameters}` : ''
  return apiRequest<ClueTimelinePage>(`/cases/${caseId}/clues${suffix}`, {}, token)
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
  payload: ReviewCluePayload,
): Promise<Clue> {
  return apiRequest<Clue>(
    `/clues/${clueId}/review`,
    { method: 'PATCH', body: JSON.stringify(payload) },
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

export function updateElderProfile(
  token: string,
  caseId: string,
  payload: UpdateElderProfilePayload,
): Promise<CaseDetail> {
  return apiRequest<CaseDetail>(
    `/cases/${caseId}/elder-profile`,
    { method: 'PATCH', body: JSON.stringify(payload) },
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
