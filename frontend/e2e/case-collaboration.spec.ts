import { expect, test, type Page } from "@playwright/test";
import {
  accounts,
  apiGet,
  apiPatch,
  apiPost,
  createCaseFixture,
  tokenFor,
  uniqueTestSuffix,
} from "./support";

async function useAccount(page: Page, token: string, path: string) {
  await page.goto("/");
  await page.evaluate((sessionToken) => {
    sessionStorage.clear();
    sessionStorage.setItem("angui.session.token", sessionToken);
  }, token);
  await page.goto(path);
}

test("a family clue moves through commander review and returns as confirmed progress", async ({
  page,
  request,
}, testInfo) => {
  const suffix = uniqueTestSuffix(testInfo);
  const fixture = await createCaseFixture(request, `collaboration-${suffix}`);
  const familyToken = await tokenFor(request, accounts.family);
  const clueContent = `Browser clue ${suffix}: north gate observation.`;

  await useAccount(page, familyToken, "/family");
  const familyCaseLink = page.getByRole("link", {
    name: fixture.displayName,
    exact: false,
  });
  await expect(page.getByRole("region", { name: "案件列表" })).toBeVisible();
  await expect(familyCaseLink).toBeVisible();
  await familyCaseLink.click();
  await page.getByRole("button", { name: /提交一条新线索/ }).click();
  await page.getByLabel("新线索内容").fill(clueContent);
  await page.getByRole("button", { name: "提交线索" }).click();
  await expect(page.getByLabel("新线索内容")).toHaveValue("");

  const familyClues = (await apiGet(
    request,
    familyToken,
    `/api/cases/${fixture.caseId}/clues`,
  )) as { items: Array<{ id: string; content: string; status: string }> };
  const submitted = familyClues.items.find((item) => item.content === clueContent);
  expect(submitted).toBeDefined();
  expect(submitted?.status).toBe("pending_review");

  const commanderToken = await tokenFor(request, accounts.commander);
  await apiPatch(request, commanderToken, `/api/clues/${submitted!.id}/review`, {
    status: "confirmed",
    reason: "The E2E commander verified this browser submission.",
  });

  await useAccount(page, commanderToken, `/command/cases/${fixture.caseId}`);
  await expect(
    page.getByRole("heading", { name: fixture.displayName }),
  ).toBeVisible();

  await useAccount(page, familyToken, "/family");
  const confirmedFamilyCaseLink = page.getByRole("link", {
    name: fixture.displayName,
    exact: false,
  });
  await expect(page.getByRole("region", { name: "案件列表" })).toBeVisible();
  await expect(confirmedFamilyCaseLink).toBeVisible();
  await confirmedFamilyCaseLink.click();
  await expect(page.getByRole("heading", { name: "公开进展" })).toBeVisible();
  await expect(page.getByText("已审核的进展更新")).toBeVisible();
});

test("a commander creates a collaboration space through the browser and its activity remains case-restricted", async ({
  page,
  request,
}, testInfo) => {
  const suffix = uniqueTestSuffix(testInfo);
  const fixture = await createCaseFixture(request, `space-${suffix}`);
  const commanderToken = await tokenFor(request, accounts.commander);
  const volunteerToken = await tokenFor(request, accounts.volunteer);
  const familyToken = await tokenFor(request, accounts.family);
  const spaceName = `East search ${suffix}`;

  await useAccount(page, commanderToken, `/command/cases/${fixture.caseId}`);
  await expect(page.getByRole("region", { name: "协作空间" })).toBeVisible();
  await page.getByLabel("协作空间名称").fill(spaceName);
  await page.getByRole("button", { name: "创建空间" }).click();

  const commanderSpaces = (await apiGet(
    request,
    commanderToken,
    `/api/cases/${fixture.caseId}/collaboration-spaces`,
  )) as Array<{ id: string; name: string; status: string }>;
  const space = commanderSpaces.find((candidate) => candidate.name === spaceName);
  expect(space).toBeDefined();
  expect(space?.status).toBe("active");

  const familyCreate = await request.post(
    `/api/cases/${fixture.caseId}/collaboration-spaces`,
    {
      headers: { Authorization: `Bearer ${familyToken}` },
      data: { name: `Forbidden ${suffix}` },
    },
  );
  // The family is a known case member but lacks the commander-only action.
  expect(familyCreate.status()).toBe(403);
  const familyList = await request.get(
    `/api/cases/${fixture.caseId}/collaboration-spaces`,
    { headers: { Authorization: `Bearer ${familyToken}` } },
  );
  expect(familyList.status()).toBe(403);
  const familySnapshot = await request.get(
    `/api/collaboration-spaces/${space!.id}/snapshot`,
    { headers: { Authorization: `Bearer ${familyToken}` } },
  );
  expect(familySnapshot.status()).toBe(403);

  const joined = (await apiPost(
    request,
    volunteerToken,
    `/api/collaboration-spaces/${space!.id}/join`,
    { location_consent: true, consent_version: "e2e-location-consent-v1" },
  )) as { member_status: string };
  expect(joined.member_status).toBe("active");

  const reportResponse = await request.post(
    `/api/collaboration-spaces/${space!.id}/voice-reports`,
    {
      headers: { Authorization: `Bearer ${volunteerToken}` },
      multipart: {
        file: {
          name: "e2e-report.webm",
          mimeType: "audio/webm",
          buffer: Buffer.from("synthetic-e2e-audio"),
        },
      },
    },
  );
  await expect(reportResponse).toBeOK();
  const report = (await reportResponse.json()) as {
    status: string;
    failed_reason: string | null;
  };
  // There is intentionally no configured ASR provider in the E2E environment.
  expect(report.status).toBe("failed");
  expect(report.failed_reason).toBe("ASR provider is not configured");

  const volunteerReports = (await apiGet(
    request,
    volunteerToken,
    `/api/collaboration-spaces/${space!.id}/voice-reports`,
  )) as Array<{ reporter_id: string; status: string; transcript?: unknown }>;
  expect(volunteerReports).toHaveLength(1);
  expect(volunteerReports[0]).toMatchObject({ status: "failed" });
  expect(volunteerReports[0]).not.toHaveProperty("transcript");

  const commanderReports = (await apiGet(
    request,
    commanderToken,
    `/api/collaboration-spaces/${space!.id}/voice-reports`,
  )) as Array<{ reporter_id: string; status: string }>;
  expect(commanderReports).toEqual(
    expect.arrayContaining([
      expect.objectContaining({ reporter_id: volunteerReports[0].reporter_id, status: "failed" }),
    ]),
  );
});
