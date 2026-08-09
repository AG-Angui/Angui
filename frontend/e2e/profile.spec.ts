import { expect, test } from "@playwright/test";
import { accounts, apiGet, tokenFor } from "./support";

test("a profile change made in the browser persists on the server", async ({
  page,
  request,
}, testInfo) => {
  const token = await tokenFor(request, accounts.family);
  const displayName = `E2E family ${testInfo.parallelIndex}`;

  await page.goto("/");
  await page.evaluate((sessionToken) => {
    sessionStorage.setItem("angui.session.token", sessionToken);
  }, token);
  await page.goto("/profile");
  await page.getByLabel("显示名称").fill(displayName);
  await page.getByRole("button", { name: "保存资料" }).click();
  await expect(page.getByText("个人资料已保存。", { exact: true })).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("显示名称")).toHaveValue(displayName);

  const profile = (await apiGet(request, token, "/api/users/me/profile")) as {
    display_name: string;
  };
  expect(profile.display_name).toBe(displayName);
});
