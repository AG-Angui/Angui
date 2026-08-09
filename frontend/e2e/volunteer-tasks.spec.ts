import { expect, test, type Page } from "@playwright/test";
import {
  accounts,
  apiGet,
  createCaseFixture,
  createTaskFixture,
  tokenFor,
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
  const fixture = await createCaseFixture(request, `volunteer-${testInfo.parallelIndex}`);
  const task = await createTaskFixture(request, fixture.caseId, String(testInfo.parallelIndex));
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
