import { apiRequest } from "./client";

export type AccountType = "member" | "learner";
export type GlobalCapability = "commander" | "volunteer" | "admin";

export interface AuthUser {
  id: string;
  email: string;
  display_name: string;
  account_type: AccountType;
  global_capabilities: GlobalCapability[];
}

export interface LoginResponse {
  token: string;
  expires_at: string;
  user: AuthUser;
}

export interface UserPreferences {
  locale: "zh-CN" | "en-US";
  reduced_motion: boolean;
}

export interface UserProfile extends AuthUser {
  team_name: string | null;
  avatar_reference: string | null;
  preferences: UserPreferences;
}

export interface UpdateUserProfilePayload {
  display_name?: string;
  avatar_reference?: string;
  preferences?: UserPreferences;
}

export function login(email: string, password: string): Promise<LoginResponse> {
  return apiRequest<LoginResponse>("/auth/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
}

export function getCurrentUser(token: string): Promise<AuthUser> {
  return apiRequest<AuthUser>("/auth/me", {}, token);
}

export function logout(token: string): Promise<void> {
  return apiRequest<void>("/auth/logout", { method: "POST" }, token);
}

export function getMyProfile(token: string): Promise<UserProfile> {
  return apiRequest<UserProfile>("/users/me/profile", {}, token);
}

export function updateMyProfile(
  token: string,
  payload: UpdateUserProfilePayload,
): Promise<UserProfile> {
  return apiRequest<UserProfile>(
    "/users/me/profile",
    { method: "PATCH", body: JSON.stringify(payload) },
    token,
  );
}
