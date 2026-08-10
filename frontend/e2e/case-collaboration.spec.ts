import { expect, test, type Page } from "@playwright/test";
import {
  accounts,
  apiGet,
  apiPatch,
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
  await page.getByRole("button", { name: fixture.displayName }).click();
  await page
    .locator("details")
    .filter({ hasText: "提交一条新线索" })
    .locator("summary")
    .click();
  await page.getByLabel("线索内容").fill(clueContent);
  await page.getByRole("button", { name: "提交线索" }).click();
  await expect(page.getByText("线索已提交，状态：待人工审核")).toBeVisible();

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
  await page.getByRole("button", { name: fixture.displayName }).click();
  await expect(page.getByText(clueContent)).toBeVisible();
});
