import { apiRequest } from "./client";

export interface AccessRequestResponse { id: string; status: string; message: string }
export interface AdminAccessRequest { id: string; email: string; display_name: string; requested_role: string; status: string; email_verified_at: string | null; created_at: string }

export function createAccessRequest(payload: { email: string; display_name: string; requested_role: string }) {
  return apiRequest<AccessRequestResponse>("/auth/access-requests", { method: "POST", body: JSON.stringify(payload) });
}
export function verifyAccessRequest(token: string) {
  return apiRequest<AccessRequestResponse>("/auth/access-requests/verify", { method: "POST", body: JSON.stringify({ token }) });
}
export function setPassword(token: string, password: string) {
  return apiRequest<void>("/auth/password-setup", { method: "POST", body: JSON.stringify({ token, password }) });
}
export function listAccessRequests(token: string) { return apiRequest<AdminAccessRequest[]>("/admin/access-requests", {}, token); }
export function reviewAccessRequest(token: string, id: string, payload: { action: string; role?: string; reason?: string }) { return apiRequest<AdminAccessRequest>(`/admin/access-requests/${id}/review`, { method: "PATCH", body: JSON.stringify(payload) }, token); }
