import { apiRequest } from './client'

export type AccountType = 'member' | 'learner'
export type GlobalCapability = 'commander' | 'volunteer' | 'admin'

export interface AuthUser {
  id: string
  email: string
  display_name: string
  account_type: AccountType
  global_capabilities: GlobalCapability[]
}

export interface LoginResponse {
  token: string
  expires_at: string
  user: AuthUser
}

export function login(email: string, password: string): Promise<LoginResponse> {
  return apiRequest<LoginResponse>('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ email, password }),
  })
}

export function getCurrentUser(token: string): Promise<AuthUser> {
  return apiRequest<AuthUser>('/auth/me', {}, token)
}

export function logout(token: string): Promise<void> {
  return apiRequest<void>('/auth/logout', { method: 'POST' }, token)
}
