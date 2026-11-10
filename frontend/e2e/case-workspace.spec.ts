import { expect, test, type Page } from "@playwright/test";
import {
  accounts,
  createCaseFixture,
  demoPassword,
  tokenFor,
  uniqueTestSuffix,
} from "./support";

async function login(page: Page, email: string) {
  await page.goto("/");
  await page.getByRole("button", { name: /家属 \/ 知情人/ }).click();
  const loginDialog = page.getByRole("dialog", { name: "家属 / 知情人登录" });
  await expect(loginDialog).toBeVisible();
  await loginDialog.getByRole("textbox", { name: "邮箱" }).fill(email);
  await loginDialog.getByRole("textbox", { name: "密码" }).fill(demoPassword);
  await loginDialog.getByRole("button", { name: "登录" }).click();
  await expect(
    page.getByRole("heading", { name: "老人走失了？" }),
  ).toBeVisible();
}

test("a case created through the live API is visible only in its member workspaces", async ({
  page,
  request,
}, testInfo) => {
  const fixture = await createCaseFixture(request, uniqueTestSuffix(testInfo));

  await login(page, accounts.family);
  await page.goto("/family");
  await expect(
    page.getByRole("heading", { level: 3, name: fixture.displayName }),
  ).toBeVisible();
  await expect(page.getByRole("link", { name: "指挥端" })).toHaveCount(0);

  // Login/logout behavior is covered independently. Install a fresh command
  // session here so this case-visibility test cannot race the previous
  // account's asynchronous remote logout.
  const commanderToken = await tokenFor(request, accounts.commander);
  await page.goto("/");
  await page.evaluate((token) => {
    sessionStorage.clear();
    sessionStorage.setItem("angui.session.token", token);
  }, commanderToken);
  await page.goto("/command");
  await expect(
    page.getByRole("button", { name: fixture.displayName }),
  ).toBeVisible();
  await expect(page.getByRole("link", { name: "指挥端" })).toBeVisible();
});
