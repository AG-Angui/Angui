export interface HealthResponse {
  status: string;
  service: string;
  version: string;
}

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? "/api";

export async function getHealth(signal?: AbortSignal): Promise<HealthResponse> {
  const response = await fetch(`${API_BASE_URL}/health`, {
    headers: { Accept: "application/json" },
    signal,
  });

  if (!response.ok) {
    throw new Error(`Health check failed with status ${response.status}`);
  }

  return (await response.json()) as HealthResponse;
}
