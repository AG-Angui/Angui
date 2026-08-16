import { apiRequest } from "./client";

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

export const listCollaborationSpaces = (token: string, caseId: string) =>
  apiRequest<CollaborationSpace[]>(`/cases/${caseId}/collaboration-spaces`, {}, token);
export const createCollaborationSpace = (token: string, caseId: string, name: string) =>
  apiRequest<CollaborationSpace>(`/cases/${caseId}/collaboration-spaces`, { method: "POST", body: JSON.stringify({ name }) }, token);
export const getSpaceSnapshot = (token: string, spaceId: string) =>
  apiRequest<SpaceSnapshot>(`/collaboration-spaces/${spaceId}/snapshot`, {}, token);
export const getSpaceEvents = (token: string, spaceId: string, afterVersion: number) =>
  apiRequest<SpaceEvent[]>(`/collaboration-spaces/${spaceId}/events?after_version=${afterVersion}`, {}, token);
export const joinCollaborationSpace = (token: string, spaceId: string, consentVersion: string) =>
  apiRequest<CollaborationSpace>(`/collaboration-spaces/${spaceId}/join`, { method: "POST", body: JSON.stringify({ location_consent: true, consent_version: consentVersion }) }, token);
export const leaveCollaborationSpace = (token: string, spaceId: string) =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/leave`, { method: "POST" }, token);
export const revokeSpaceLocationConsent = (token: string, spaceId: string) =>
  apiRequest<void>(`/collaboration-spaces/${spaceId}/location-consents/me`, { method: "DELETE" }, token);
