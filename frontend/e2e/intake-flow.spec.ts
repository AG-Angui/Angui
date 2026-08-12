import { expect, test, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import {
  accounts,
  apiPost,
  apiSsePost,
  tokenFor,
  uniqueTestSuffix,
} from "./support";

async function useFamilySession(page: Page, token: string) {
  await page.goto("/");
  await page.evaluate((sessionToken) => {
    sessionStorage.clear();
    sessionStorage.setItem("angui.session.token", sessionToken);
  }, token);
  await page.goto("/family");
}

test("the intake confirmation workflow creates one browser-visible case only after AI acknowledgement", async ({
  page,
  request,
}, testInfo) => {
  test.setTimeout(120_000);
  const token = await tokenFor(request, accounts.family);
  const displayName = `E2E intake ${uniqueTestSuffix(testInfo)}`;
  const profile = {
    display_name: displayName,
    age: 76,
    gender: "female",
    physical_description: "Synthetic intake profile.",
    clothing_description: "Blue jacket.",
    health_notes: "Synthetic health note.",
    last_seen_at: "2026-07-25T09:00:00Z",
    last_seen_location: "Synthetic community gate",
  };
  const session = (await apiPost(request, token, "/api/intake-sessions", {})) as {
    id: string;
  };
  for (const [field, answer] of [
    ["basic_information", "姓名：端到端测试老人；身高：168厘米；特征描述：戴蓝色帽子"],
    ["last_seen", "Synthetic last seen location."],
    ["suspicious_motive", "No known suspicious motive."],
    ["police_report_status", "已报警"],
    ["family_phone", "13800138000"],
    ["health_status", "Synthetic health status supplied by the family."],
  ]) {
    await apiPost(request, token, `/api/intake-sessions/${session.id}/answers`, {
      field,
      answer,
    });
  }
  const photoResponse = await request.post(
    `/api/intake-sessions/${session.id}/photos`,
    {
      headers: { Authorization: `Bearer ${token}` },
      multipart: {
        file: {
          name: "e2e-missing-person.png",
          mimeType: "image/png",
          buffer: await readFile("../assets/brand/angui-mark-128.png"),
        },
      },
    },
  );
  await expect(photoResponse).toBeOK();
  const uploadedPhoto = (await photoResponse.json()) as {
    id: string;
    content_type: string;
  };
  expect(uploadedPhoto.content_type).toBe("image/png");
  const privatePreview = await request.get(
    `/api/intake-sessions/${session.id}/photos/${uploadedPhoto.id}`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  await expect(privatePreview).toBeOK();
  expect(privatePreview.headers()["cache-control"]).toContain("private");
  expect(privatePreview.headers()["x-content-type-options"]).toBe("nosniff");

  const initialReview = await apiSsePost<{
    issues: Array<{ id: string }>;
    status: string;
  }>(request, token, `/api/intake-sessions/${session.id}/ai-initial-review`, {
    profile,
  });
  expect(initialReview.status).toBe("awaiting_family_review");
  const acknowledgement = (await apiPost(
    request,
    token,
    `/api/intake-sessions/${session.id}/ai-initial-review/acknowledge`,
    {
      human_confirmed: true,
      confirmed_issue_ids: initialReview.issues.map((issue) => issue.id),
    },
  )) as { ready_for_second_confirmation: boolean };
  expect(acknowledgement.ready_for_second_confirmation).toBe(true);

  const firstConfirmation = (await apiPost(
    request,
    token,
    `/api/intake-sessions/${session.id}/confirm`,
    { human_confirmed: true, profile },
  )) as { case_id: string; confirmation_status: string };
  expect(firstConfirmation.confirmation_status).toBe(
    "human_confirmed_after_ai_initial_review",
  );
  const secondConfirmation = (await apiPost(
    request,
    token,
    `/api/intake-sessions/${session.id}/confirm`,
    { human_confirmed: true, profile },
  )) as { case_id: string };
  expect(secondConfirmation.case_id).toBe(firstConfirmation.case_id);

  await useFamilySession(page, token);
  await expect(
    page.getByRole("link", { name: new RegExp(displayName) }),
  ).toBeVisible();
});
