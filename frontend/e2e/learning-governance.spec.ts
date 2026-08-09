import { expect, test, type Page } from "@playwright/test";
import {
  accounts,
  apiPost,
  createPublishedLearningResource,
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

test("published learning content is visible to learners and withdrawal revokes it", async ({
  page,
  request,
}, testInfo) => {
  const resource = await createPublishedLearningResource(
    request,
    String(testInfo.parallelIndex),
  );
  const learnerToken = await tokenFor(request, accounts.learner);

  await useAccount(page, learnerToken, "/learning");
  await expect(page.getByText(resource.title).first()).toBeVisible();

  const adminToken = await tokenFor(request, accounts.admin);
  await apiPost(
    request,
    adminToken,
    `/api/admin/learning/resources/${resource.resourceId}/withdraw`,
    { reason: "E2E withdrawal verifies learner access revocation." },
  );
  await page.reload();
  await expect(page.getByText(resource.title)).toHaveCount(0);
});
