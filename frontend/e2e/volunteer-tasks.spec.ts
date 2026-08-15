import { expect, test, type Page } from "@playwright/test";
import {
  accounts,
  apiGet,
  apiPost,
  createCaseFixture,
  createClueFixture,
  createTaskFixture,
  confirmClueFixture,
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

test("a volunteer can apply for an open task and the application is persisted", async ({
  page,
  request,
}, testInfo) => {
  const suffix = uniqueTestSuffix(testInfo);
  const fixture = await createCaseFixture(request, `volunteer-${suffix}`);
  const task = await createTaskFixture(request, fixture.caseId, suffix);
  const volunteerToken = await tokenFor(request, accounts.volunteer);

  await useAccount(page, volunteerToken, "/volunteer");
  await expect(page.getByText(task.title)).toBeVisible();
  await page.getByRole("button", { name: "申请协作" }).click();
  await expect(page.getByText("任务申请已提交，等待指挥人员审核。")).toBeVisible();

  const commanderToken = await tokenFor(request, accounts.commander);
  const applications = (await apiGet(
    request,
    commanderToken,
    `/api/tasks/${task.taskId}/applications`,
  )) as Array<{ volunteer_user_id: string; status: string }>;
  expect(applications).toEqual(
    expect.arrayContaining([
      expect.objectContaining({ status: "pending" }),
    ]),
  );
});

test("a volunteer sees only the accepted task's source clue and text location", async ({
  page,
  request,
}, testInfo) => {
  test.setTimeout(90_000);
  const suffix = uniqueTestSuffix(testInfo);
  const fixture = await createCaseFixture(request, `volunteer-context-${suffix}`);
  const clue = await createClueFixture(request, fixture.caseId, suffix);
  await confirmClueFixture(request, clue.clueId);
  const volunteerToken = await tokenFor(request, accounts.volunteer);
  const volunteer = (await apiGet(request, volunteerToken, "/api/auth/me")) as {
    id: string;
  };
  const commanderToken = await tokenFor(request, accounts.commander);
  const taskTitle = `E2E assigned task ${suffix}`;

  await apiPost(request, commanderToken, `/api/cases/${fixture.caseId}/tasks`, {
    source_clue_id: clue.clueId,
    volunteer_user_id: volunteer.id,
    title: taskTitle,
    objective: "Verify the assigned synthetic report.",
    area_text: "Test park north gate",
    latitude: null,
    longitude: null,
    due_at: "2099-07-27T12:00:00Z",
    background: "Commander-only background must stay hidden.",
    risk_level: "low",
    risk_notes: "Remain in public areas.",
    safety_briefing: "Keep contact with the commander.",
    expected_feedback: "Submit a factual report.",
  });

  await useAccount(page, volunteerToken, "/volunteer");

  await expect(page.getByText(taskTitle)).toBeVisible();
  await expect(page.getByText(clue.content).first()).toBeVisible();
  await expect(page.getByText(/Test park north gate/).first()).toBeVisible();
  await expect(page.getByText("Synthetic test data.")).toHaveCount(0);
  await expect(page.getByText(accounts.family)).toHaveCount(0);
});
