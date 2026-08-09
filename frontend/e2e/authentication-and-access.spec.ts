import { expect, test, type Page } from "@playwright/test";

const password = "e2e-demo-password";

async function loginAs(page: Page, email: string) {
  await page.goto("/");
  await page.getByRole("textbox", { name: "邮箱" }).fill(email);
  await page.getByRole("textbox", { name: "密码" }).fill(password);
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "行动总览" })).toBeVisible();
}

test("the browser can reach the live API through Vite's proxy", async ({ request }) => {
  const response = await request.get("/api/health");

  await expect(response).toBeOK();
  await expect(await response.json()).toEqual({
    status: "ok",
    service: "angui-api",
    version: "0.1.0",
  });
});

test("invalid credentials never establish a browser session", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("textbox", { name: "邮箱" }).fill("family@demo.invalid");
  await page.getByRole("textbox", { name: "密码" }).fill("wrong-password");
  await page.getByRole("button", { name: "登录" }).click();

  await expect(page.getByRole("alert")).toContainText("邮箱或密码错误");
  await page.reload();
  await expect(page.getByRole("heading", { name: "账号登录" })).toBeVisible();
});

test("a family member can log in and logout, but cannot open the commander workspace", async ({
  page,
}) => {
  await loginAs(page, "family@demo.invalid");

  await expect(page.getByRole("heading", { name: "行动总览" })).toBeVisible();
  await expect(page.getByRole("link", { name: "家属端" })).toBeVisible();
  await expect(page.getByRole("link", { name: "指挥端" })).toHaveCount(0);

  await page.goto("/command");
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByRole("heading", { name: "行动总览" })).toBeVisible();

  await page.getByRole("button", { name: "退出登录" }).first().click();
  await expect(page.getByRole("heading", { name: "账号登录" })).toBeVisible();
});

test("a commander can open command work but is redirected away from family-only work", async ({
  page,
}) => {
  await loginAs(page, "commander@demo.invalid");

  await page.goto("/command");
  await expect(page.getByRole("region", { name: "案件列表" })).toBeVisible();
  await expect(page.getByRole("link", { name: "指挥端" })).toBeVisible();
  await expect(page.getByRole("link", { name: "家属端" })).toHaveCount(0);

  await page.goto("/family");
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByRole("heading", { name: "行动总览" })).toBeVisible();
});
