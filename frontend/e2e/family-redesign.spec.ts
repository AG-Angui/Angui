import { expect, test, type Page } from "@playwright/test";

const password = "e2e-demo-password";

async function loginAsFamily(page: Page) {
  await page.goto("/");
  await page.getByRole("textbox", { name: "邮箱" }).fill("family@demo.invalid");
  await page.getByRole("textbox", { name: "密码" }).fill(password);
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "行动总览" })).toBeVisible();
}

test.describe("family redesign", () => {
  test("family can open the request entry and desktop intake shell", async ({ page }) => {
    await loginAsFamily(page);
    await page.goto("/family");
    await expect(page.getByRole("heading", { name: "说清楚情况，安心看进展" })).toBeVisible();
    await page.getByRole("link", { name: /开始求助/ }).click();
    await expect(page).toHaveURL(/\/family\/intake$/);
    await expect(page.getByRole("navigation", { name: "建案步骤" })).toBeVisible();
    await expect(page.getByText("老人画像预览")).toBeVisible();
  });

  test("family mobile keeps a single-column entry and does not expose command routes", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await loginAsFamily(page);
    await page.goto("/family");
    await expect(page.getByRole("link", { name: /开始求助/ })).toBeVisible();
    await page.goto("/family/intake");
    await expect(page.getByText("每一步都可标记“不知道”；确认提交前不会创建正式案件。")).toBeVisible();
    await page.goto("/command");
    await expect(page).toHaveURL(/\/$/);
  });
});
