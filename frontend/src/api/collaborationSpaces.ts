import { apiRequest } from "./client";
import type { ClueDraftCandidate } from "./cases";

export interface CollaborationSpace {
  id: string;
  case_id: string;
  name: string;
  status: "active" | "archived";
  created_by_user_id: string;
  created_at: string;
  archived_at: string | null;
  current_version: number;
  member_status: "active" | "left" | null;
}

export interface SpaceMember {
  id: string;
  user_id: string;
  display_name: string;
  role: "commander" | "volunteer";
  status: "active" | "left";
  joined_at: string;
  left_at: string | null;
  location_consent_granted: boolean;
}

export interface SpaceSnapshot { space: CollaborationSpace; members: SpaceMember[]; version: number }
export interface SpaceEvent { event_id: string; space_id: string; case_id: string; event_type: string; version: number; occurred_at: string; visibility_scope: string; payload: Record<string, unknown> }
export interface SpaceLocation { id: string; user_id: string; latitude: number; longitude: number; accuracy_meters: number; captured_at: string }
export interface SpaceMessage { id: string; sender_id: string; sender_display_name: string; message_type: "text" | "broadcast"; content: string; sent_at: string; recalled_at: string | null }
export interface VoiceTranscript { content: string; provider: string; status: "completed" | "failed"; created_at: string }
export interface VoiceReport { id: string; reporter_id: string; content_type: string; byte_size: number; status: "uploaded" | "transcribing" | "transcribed" | "draft_ready" | "failed" | "reviewed"; created_at: string; failed_reason: string | null; transcript?: VoiceTranscript }
export interface VoiceRoomTicket { url: string; token: string; expires_at: number; ice_servers: RTCIceServer[] }
export interface VoiceCandidate {
  id: string; voice_report_id: string | null; intercom_recording_id: string | null;
  source_type: string; status: string; retry_count: number; failure_reason: string | null;
  clue_draft_id: string | null; discoverer_user_id: string | null; submitted_by_user_id: string;
  can_confirm_discoverer: boolean;
  returned_for_revision: boolean; review_note: string | null;
  editable_candidate: ClueDraftCandidate | null;
  transcript?: string | null;
}

export const listCollaborationSpaces = (token: string, caseId: string) =>
  apiRequest<CollaborationSpace[]>(`/cases/${caseId}/collaboration-spaces`, {}, token);
export const createCollaborationSpace = (token: string, caseId: string, name: string) =>
  apiRequest<CollaborationSpace>(`/cases/${caseId}/collaboration-spaces`, { method: "POST", body: JSON.stringify({ name }) }, token);
export const getSpaceSnapshot = (token: string, spaceId: string) =>
  apiRequest<SpaceSnapshot>(`/collaboration-spaces/${spaceId}/snapshot`, {}, token);
export const getSpaceEvents = (token: string, spaceId: string, afterVersion: number) =>
  apiRequest<SpaceEvent[]>(`/collaboration-spaces/${spaceId}/events?after_version=${afterVersion}`, {}, token);
export const getSpaceSocketTicket = (token: string, spaceId: string) =>
  apiRequest<{ ticket: string; expires_in_seconds: number }>(`/collaboration-spaces/${spaceId}/events/ticket`, { method: "POST" }, token);
export const joinCollaborationSpace = (token: string, spaceId: string, consentVersion: string) =>
  apiRequest<CollaborationSpace>(`/collaboration-spaces/${spaceId}/join`, { method: "POST", body: JSON.stringify({ location_consent: true, consent_version: consentVersion }) }, token);
export const leaveCollaborationSpace = (token: string, spaceId: string) =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/leave`, { method: "POST" }, token);
export const archiveCollaborationSpace = (token: string, spaceId: string) =>
  apiRequest<CollaborationSpace>(`/collaboration-spaces/${spaceId}/archive`, { method: "POST" }, token);
export const revokeSpaceLocationConsent = (token: string, spaceId: string) =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/location-consents/me`, { method: "DELETE" }, token);
export const recordSpaceLocation = (token: string, spaceId: string, location: Omit<SpaceLocation, "id" | "user_id"> & { operation_id: string }) =>
  apiRequest<SpaceLocation>(`/collaboration-spaces/${spaceId}/locations`, { method: "POST", body: JSON.stringify(location) }, token);
export const listMemberTrack = (token: string, spaceId: string, userId: string) =>
  apiRequest<SpaceLocation[]>(`/collaboration-spaces/${spaceId}/members/${userId}/track`, {}, token);
export const listLatestSpaceLocations = (token: string, spaceId: string) =>
  apiRequest<SpaceLocation[]>(`/collaboration-spaces/${spaceId}/locations/latest`, {}, token);
export const listSpaceMessages = (token: string, spaceId: string) =>
  apiRequest<SpaceMessage[]>(`/collaboration-spaces/${spaceId}/messages`, {}, token);
export const sendSpaceMessage = (token: string, spaceId: string, content: string, message_type: "text" | "broadcast" = "text") =>
  apiRequest<SpaceMessage>(`/collaboration-spaces/${spaceId}/messages`, { method: "POST", body: JSON.stringify({ content, message_type }) }, token);
export const acknowledgeSpaceMessage = (token: string, spaceId: string, messageId: string, status: "delivered" | "acknowledged" = "delivered") =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/messages/${messageId}/ack`, { method: "POST", body: JSON.stringify({ status }) }, token);
export const listVoiceReports = (token: string, spaceId: string) =>
  apiRequest<VoiceReport[]>(`/collaboration-spaces/${spaceId}/voice-reports`, {}, token);
export const uploadVoiceReport = (token: string, spaceId: string, file: File) => {
  const body = new FormData();
  body.append("file", file, file.name);
  return apiRequest<VoiceReport>(`/collaboration-spaces/${spaceId}/voice-reports`, { method: "POST", body }, token);
};
export const getVoiceRoomTicket = (token: string, spaceId: string) =>
  apiRequest<VoiceRoomTicket>(`/collaboration-spaces/${spaceId}/voice-room/ticket`, { method: "POST" }, token);
export const setVoiceFloor = (token: string, spaceId: string, active: boolean) =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/voice-room/floor`, { method: "POST", body: JSON.stringify({ active }) }, token);
export const uploadIntercomRecording = (token: string, spaceId: string, file: File, startedAt: string, endedAt: string) => {
  const body = new FormData();
  body.append("file", file, file.name);
  const times = new URLSearchParams({ started_at: startedAt, ended_at: endedAt });
  return apiRequest<{ id: string; candidate_id: string; status: string }>(
    `/collaboration-spaces/${spaceId}/recordings?${times}`, { method: "POST", body }, token);
};
export const listVoiceCandidates = (token: string, spaceId: string) =>
  apiRequest<VoiceCandidate[]>(`/collaboration-spaces/${spaceId}/voice-candidates`, {}, token);
export const retryVoiceCandidate = (token: string, spaceId: string, id: string) =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/voice-candidates/${id}/retry`, { method: "POST" }, token);
export const confirmVoiceDiscoverer = (token: string, spaceId: string, id: string, discovererUserId: string) =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/voice-candidates/${id}/discoverer`, { method: "POST", body: JSON.stringify({ discoverer_user_id: discovererUserId }) }, token);
export const returnVoiceCandidate = (token: string, spaceId: string, id: string, reason: string) =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/voice-candidates/${id}/return`, { method: "POST", body: JSON.stringify({ reason }) }, token);
export const resubmitVoiceCandidate = (token: string, spaceId: string, id: string, candidate: ClueDraftCandidate) =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/voice-candidates/${id}/resubmit`, { method: "POST", body: JSON.stringify(candidate) }, token);
export const mergeVoiceCandidate = (token: string, spaceId: string, id: string, targetClueId: string, reason: string) =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/voice-candidates/${id}/merge`, { method: "POST", body: JSON.stringify({ target_clue_id: targetClueId, reason }) }, token);
