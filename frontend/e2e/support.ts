import { expect, type APIRequestContext, type TestInfo } from "@playwright/test";

export const demoPassword = "e2e-demo-password";
export const accounts = {
  family: "family@demo.invalid",
  commander: "commander@demo.invalid",
  volunteer: "volunteer@demo.invalid",
  learner: "learner@demo.invalid",
  admin: "admin@demo.invalid",
} as const;

const localRunId = `local-${process.pid}-${Date.now()}`;

export function uniqueTestSuffix(testInfo: TestInfo) {
  const runId = process.env.GITHUB_RUN_ID
    ? `${process.env.GITHUB_RUN_ID}-${process.env.GITHUB_RUN_ATTEMPT ?? "1"}`
    : localRunId;
  const testId = testInfo.testId.replace(/[^a-zA-Z0-9-]/g, "-").slice(-40);
  return `${runId}-${testId}-r${testInfo.retry}`;
}

export async function tokenFor(request: APIRequestContext, email: string) {
  const response = await request.post("/api/auth/login", {
    data: { email, password: demoPassword },
  });
  await expect(response).toBeOK();
  return (await response.json()).token as string;
}

export async function apiPost(
  request: APIRequestContext,
  token: string,
  path: string,
  data?: unknown,
) {
  const response = await request.post(path, {
    headers: { Authorization: `Bearer ${token}` },
    data,
  });
  await expect(response).toBeOK();
  return response.json();
}

export async function apiPatch(
  request: APIRequestContext,
  token: string,
  path: string,
  data?: unknown,
) {
  const response = await request.patch(path, {
    headers: { Authorization: `Bearer ${token}` },
    data,
  });
  await expect(response).toBeOK();
  return response.json();
}

export async function apiGet(
  request: APIRequestContext,
  token: string,
  path: string,
) {
  const response = await request.get(path, {
    headers: { Authorization: `Bearer ${token}` },
  });
  await expect(response).toBeOK();
  return response.json();
}

export async function apiSsePost<T>(
  request: APIRequestContext,
  token: string,
  path: string,
  data: unknown,
): Promise<T> {
  const response = await request.post(path, {
    headers: { Authorization: `Bearer ${token}` },
    data,
  });
  await expect(response).toBeOK();
  const body = await response.text();
  const completed = [...body.matchAll(/^data:\s*(.+)$/gm)].at(-1)?.[1];
  if (!completed) throw new Error(`SSE completion missing for ${path}`);
  return JSON.parse(completed) as T;
}

export async function createCaseFixture(request: APIRequestContext, suffix: string) {
  const familyToken = await tokenFor(request, accounts.family);
  const caseDetail = await apiPost(request, familyToken, "/api/cases", {
    display_name: `E2E case ${suffix}`,
    age: 76,
    gender: "female",
    physical_description: "Synthetic profile for end-to-end testing.",
    clothing_description: "Blue jacket.",
    health_notes: "Synthetic test data.",
    last_seen_at: "2026-07-13T09:00:00Z",
    last_seen_location: "Test park north gate",
  });
  const caseId = caseDetail.id as string;
  await apiPost(request, familyToken, `/api/cases/${caseId}/members`, {
    email: accounts.commander,
    case_role: "commander",
  });
  const commanderToken = await tokenFor(request, accounts.commander);
  await apiPost(request, commanderToken, `/api/cases/${caseId}/members`, {
    email: accounts.volunteer,
    case_role: "volunteer",
  });
  return { caseId, displayName: caseDetail.elder_profile.display_name as string };
}

export async function createClueFixture(
  request: APIRequestContext,
  caseId: string,
  suffix: string,
) {
  const familyToken = await tokenFor(request, accounts.family);
  const content = `E2E clue ${suffix}: sighting near the north gate.`;
  const clue = await apiPost(request, familyToken, `/api/cases/${caseId}/clues`, {
    source: "family report",
    content,
    occurred_at: "2026-07-13T09:30:00Z",
    location_text: "Test park north gate",
    location_precision: "approximate",
    source_type: "manual_report",
  });
  return { clueId: clue.id as string, content };
}

export async function confirmClueFixture(
  request: APIRequestContext,
  clueId: string,
) {
  const commanderToken = await tokenFor(request, accounts.commander);
  return apiPatch(request, commanderToken, `/api/clues/${clueId}/review`, {
    status: "confirmed",
    reason: "Verified by the E2E commander fixture.",
  });
}

export async function createTaskFixture(
  request: APIRequestContext,
  caseId: string,
  suffix: string,
) {
  const clue = await createClueFixture(request, caseId, suffix);
  await confirmClueFixture(request, clue.clueId);
  const commanderToken = await tokenFor(request, accounts.commander);
  const title = `E2E task ${suffix}`;
  const task = await apiPost(request, commanderToken, `/api/cases/${caseId}/tasks`, {
    source_clue_id: clue.clueId,
    title,
    objective: "Verify the synthetic report and submit observations.",
    area_text: "Test park north gate",
    latitude: 31.2,
    longitude: 121.5,
    due_at: "2099-07-27T12:00:00Z",
    background: "A reviewed E2E clue needs field verification.",
    risk_level: "medium",
    risk_notes: "Remain in public areas.",
    safety_briefing: "Keep contact with the commander.",
    expected_feedback: "Submit a factual report.",
  });
  return { taskId: task.id as string, title, clue };
}

export async function createPublishedLearningResource(
  request: APIRequestContext,
  suffix: string,
) {
  const title = `E2E learning resource ${suffix}`;
  const adminToken = await tokenFor(request, accounts.admin);
  const resource = await apiPost(request, adminToken, "/api/admin/learning/resources", {
    title,
    summary: "Synthetic learning resource for browser verification.",
    content: "This content exists only to validate the publication workflow.",
    resource_type: "prevention",
    tags: ["e2e", "test"],
    source_name: "E2E approved source",
    source_url: "https://example.invalid/e2e-source",
    visibility: "public",
    effective_at: "2020-01-01T00:00:00.000Z",
    permitted_use: "public_information",
    submission_reason: "E2E governance workflow verification.",
  });
  const resourceId = resource.id as string;
  const commanderToken = await tokenFor(request, accounts.commander);
  const volunteerToken = await tokenFor(request, accounts.volunteer);
  await apiPost(
    request,
    commanderToken,
    `/api/admin/learning/resources/${resourceId}/deidentify`,
    { reason: "Independent E2E deidentification review." },
  );
  await apiPost(
    request,
    volunteerToken,
    `/api/admin/learning/resources/${resourceId}/review`,
    { reason: "Independent E2E content review." },
  );
  await apiPost(
    request,
    adminToken,
    `/api/admin/learning/resources/${resourceId}/publish`,
    { reason: "Independent E2E publication approval." },
  );
  return { resourceId, title };
}
