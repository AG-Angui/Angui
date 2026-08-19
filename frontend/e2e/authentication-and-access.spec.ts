import { expect, test, type Page } from "@playwright/test";

const password = "e2e-demo-password";

async function loginAs(page: Page, email: string) {
  await page.goto("/", { waitUntil: "networkidle" });
  // Each test gets an isolated context, but clearing storage here also makes
  // retries deterministic when the previous attempt stopped mid-redirect.
  await page.evaluate(() => {
    sessionStorage.clear();
    localStorage.clear();
  });
  await page.goto("/", { waitUntil: "networkidle" });

  const emailInput = page.getByRole("textbox", { name: "邮箱" });
  const passwordInput = page.getByRole("textbox", { name: "密码" });
  await expect(emailInput).toBeVisible({ timeout: 30_000 });
  await emailInput.fill(email);
  await passwordInput.fill(password);

  const loginResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/auth/login") &&
      response.request().method() === "POST" &&
      response.ok(),
  );
  await page.getByRole("button", { name: "登录" }).click();
  await loginResponse;
  await expect(page.getByRole("heading", { name: "行动总览" })).toBeVisible({
    timeout: 30_000,
  });
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
  await expect(page.getByRole("heading", { name: "安归｜身份登录" })).toBeVisible();
});

test("the redesigned login keeps role guidance and submits an access request", async ({ page }, testInfo) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "面向失智老人走失搜救的协同入口" })).toBeVisible();
  await page.getByRole("button", { name: /志愿者/ }).click();
  await expect(page.getByRole("button", { name: /志愿者/ })).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("link", { name: "申请访问" }).click();
  await expect(page).toHaveURL(/\/access-request$/);
  await expect(page.getByRole("heading", { name: "申请访问安归" })).toBeVisible();
  await page.getByRole("textbox", { name: "姓名" }).fill("E2E access applicant");
  await page.getByRole("textbox", { name: "邮箱" }).fill(`access-${testInfo.testId.replace(/[^a-z0-9]/gi, "-")}@example.invalid`);
  await page.getByRole("button", { name: "发送验证邮件" }).click();
  await expect(page.getByRole("status")).toContainText("如果邮箱可以申请访问");
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
  await expect(page.getByRole("heading", { name: "安归｜身份登录" })).toBeVisible();
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
