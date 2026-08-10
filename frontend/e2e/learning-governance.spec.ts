import { expect, test, type Page } from "@playwright/test";
import {
  accounts,
  apiGet,
  apiPost,
  createPublishedLearningResource,
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

test("published learning content is visible to learners and withdrawal revokes it", async ({
  page,
  request,
}, testInfo) => {
  const resource = await createPublishedLearningResource(
    request,
    uniqueTestSuffix(testInfo),
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

test("learner category governance carries a draft through publication and filtering", async ({
  page,
  request,
}, testInfo) => {
  const suffix = uniqueTestSuffix(testInfo);
  const categoryName = `E2E category ${suffix}`;
  const resourceTitle = `E2E categorized learning ${suffix}`;
  const learnerToken = await tokenFor(request, accounts.learner);
  const adminToken = await tokenFor(request, accounts.admin);

  const proposal = await apiPost(
    request,
    learnerToken,
    "/api/learning/categories/proposals",
    {
      name: categoryName,
      submission_reason: "E2E verifies learner category governance.",
    },
  );
  expect(proposal.status).toBe("pending");
  const categoryId = proposal.id as string;

  const enabled = await apiPost(
    request,
    adminToken,
    `/api/admin/learning/categories/${categoryId}/enable`,
    { reason: "E2E category is suitable for the learning center." },
  );
  expect(enabled.status).toBe("enabled");

  await useAccount(page, learnerToken, "/learning");
  const draftSection = page
    .locator("section")
    .filter({ has: page.getByRole("heading", { name: "提交学习资源草稿" }) });
  await expect(draftSection).toBeVisible();
  await page.getByLabel("草稿标题").fill(resourceTitle);
  await page.getByLabel("草稿来源名称").fill("E2E learning group");
  await page.getByLabel("草稿标签").fill("e2e-tag, onboarding");
  await page.getByLabel("摘要").fill("A categorized E2E learning summary.");
  await page.getByLabel("正文").fill("A categorized E2E learning body.");
  await page.getByLabel("提交理由").fill("E2E verifies the learner draft workflow.");
  await draftSection.locator("select").selectOption(categoryId);
  await page.getByRole("button", { name: "提交草稿" }).click();
  await expect(page.getByRole("status")).toContainText("草稿已提交");

  const managedResources = await apiGet(
    request,
    adminToken,
    "/api/admin/learning/resources",
  );
  const draft = managedResources.find(
    (resource: { title: string }) => resource.title === resourceTitle,
  ) as {
    id: string;
    category: { id: string } | null;
    lifecycle: { state: string };
  };
  expect(draft).toBeTruthy();
  expect(draft.category?.id).toBe(categoryId);
  expect(draft.lifecycle.state).toBe("submitted");

  const commanderToken = await tokenFor(request, accounts.commander);
  const volunteerToken = await tokenFor(request, accounts.volunteer);
  await apiPost(
    request,
    commanderToken,
    `/api/admin/learning/resources/${draft.id}/deidentify`,
    { reason: "E2E independent de-identification." },
  );
  await apiPost(
    request,
    volunteerToken,
    `/api/admin/learning/resources/${draft.id}/review`,
    { reason: "E2E independent content review." },
  );
  await apiPost(
    request,
    adminToken,
    `/api/admin/learning/resources/${draft.id}/publish`,
    { reason: "E2E publication approval." },
  );

  await page.reload();
  const publishedCard = page.locator("article").filter({ hasText: resourceTitle });
  await expect(publishedCard).toHaveCount(1);
  await expect(publishedCard).toContainText(categoryName);
  await expect(publishedCard).toContainText("#e2e-tag");

  const categoryFilterResponse = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      response.request().method() === "GET" &&
      url.pathname === "/api/learning/resources" &&
      url.searchParams.get("category_id") === categoryId &&
      !url.searchParams.has("tag")
    );
  });
  await page.getByLabel("分类筛选").selectOption(categoryId);
  const categoryResources = (await (await categoryFilterResponse).json()) as Array<{
    id: string;
    category: { id: string } | null;
  }>;
  expect(categoryResources).toEqual([
    expect.objectContaining({
      id: draft.id,
      category: expect.objectContaining({ id: categoryId }),
    }),
  ]);
  await expect(publishedCard).toBeVisible();

  const tagFilterResponse = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      response.request().method() === "GET" &&
      url.pathname === "/api/learning/resources" &&
      url.searchParams.get("category_id") === categoryId &&
      url.searchParams.get("tag") === "e2e-tag"
    );
  });
  await page.getByLabel("标签筛选").selectOption("e2e-tag");
  const tagResources = (await (await tagFilterResponse).json()) as Array<{
    id: string;
    tags: string[];
  }>;
  expect(tagResources).toEqual([
    expect.objectContaining({ id: draft.id, tags: ["e2e-tag", "onboarding"] }),
  ]);
  await expect(publishedCard).toBeVisible();
});
