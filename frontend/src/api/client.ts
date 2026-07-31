const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? "/api";
export const SESSION_EXPIRED_EVENT = "angui:session-expired";

interface ApiErrorPayload {
  error?: {
    code?: string;
    message?: string;
  };
}

export class ApiClientError extends Error {
  status: number;
  code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = "ApiClientError";
    this.status = status;
    this.code = code;
  }
}

function userMessageFor(
  status: number,
  code: string,
  hasSession = false,
): string {
  if (status === 0) return "网络连接失败，请检查服务连接后重试。";
  if (status === 400 || code === "validation_error")
    return "提交的信息不符合要求，请检查后重试。";
  if (status === 401 || code === "unauthorized") {
    return hasSession
      ? "登录状态已失效，请重新登录。"
      : "邮箱或密码错误，请重新输入。";
  }
  if (status === 403 || code === "forbidden") return "你没有执行此操作的权限。";
  if (status === 404 || code === "not_found")
    return "未找到可访问的资源，它可能已不存在或你没有访问权限。";
  if (status === 409 || code === "conflict")
    return "当前数据状态已变化，请刷新后再试。";
  if (status === 429 || code === "rate_limited")
    return "请求过于频繁，请稍后再试。";
  return "服务暂时不可用，请稍后重试。";
}

function notifySessionExpired() {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new Event(SESSION_EXPIRED_EVENT));
  }
}

export async function apiRequest<T>(
  path: string,
  options: RequestInit = {},
  token?: string | null,
): Promise<T> {
  const headers = new Headers(options.headers);
  headers.set("Accept", "application/json");
  if (
    options.body &&
    !(options.body instanceof FormData) &&
    !headers.has("Content-Type")
  ) {
    headers.set("Content-Type", "application/json");
  }
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  let response: Response;
  try {
    response = await fetch(`${API_BASE_URL}${path}`, {
      ...options,
      headers,
    });
  } catch {
    throw new ApiClientError(
      0,
      "network_error",
      userMessageFor(0, "network_error"),
    );
  }

  if (!response.ok) {
    let payload: ApiErrorPayload = {};
    try {
      payload = (await response.json()) as ApiErrorPayload;
    } catch {
      // The status code still provides a useful fallback when a proxy returns HTML.
    }
    const code = payload.error?.code ?? "request_failed";
    if (response.status === 401 && token) notifySessionExpired();
    throw new ApiClientError(
      response.status,
      code,
      userMessageFor(response.status, code, Boolean(token)),
    );
  }

  if (response.status === 204) {
    return undefined as T;
  }
  return (await response.json()) as T;
}
