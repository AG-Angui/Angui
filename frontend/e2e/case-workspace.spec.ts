import { expect, test, type Page } from "@playwright/test";
import { accounts, createCaseFixture, demoPassword } from "./support";

async function login(page: Page, email: string) {
  await page.goto("/");
  await page.getByRole("textbox", { name: "邮箱" }).fill(email);
  await page.getByRole("textbox", { name: "密码" }).fill(demoPassword);
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "行动总览" })).toBeVisible();
}

test("a case created through the live API is visible only in its member workspaces", async ({
  page,
  request,
}, testInfo) => {
  const fixture = await createCaseFixture(request, String(testInfo.parallelIndex));

  await login(page, accounts.family);
  await page.goto("/family");
  await expect(
    page.getByRole("button", { name: fixture.displayName }),
  ).toBeVisible();
  await expect(page.getByRole("link", { name: "指挥端" })).toHaveCount(0);

  await page.getByRole("button", { name: "退出登录" }).first().click();
  await login(page, accounts.commander);
  await page.goto("/command");
  await expect(
    page.getByRole("button", { name: fixture.displayName }),
  ).toBeVisible();
  await expect(page.getByRole("link", { name: "指挥端" })).toBeVisible();
});
