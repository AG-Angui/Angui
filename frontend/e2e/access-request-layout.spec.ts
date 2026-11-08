import { expect, test } from "@playwright/test";

const viewports = [
  { width: 390, height: 844 },
  { width: 768, height: 1024 },
  { width: 1024, height: 900 },
  { width: 1440, height: 900 },
];

for (const viewport of viewports) {
  test(`access request layout, keyboard and receipt at ${viewport.width}px`, async ({ page }, testInfo) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
    await page.setViewportSize(viewport);
    let submitted: unknown;
    await page.route("**/api/auth/access-requests", async (route) => {
      submitted = route.request().postDataJSON();
      await route.fulfill({ json: { id: "layout-receipt", status: "pending_verification", message: "如果邮箱可以申请访问，我们会发送一封验证邮件，请查收后继续。" } });
    });
    await page.goto("/access-request?role=volunteer");
    await expect(page.getByRole("heading", { name: "申请访问安归" })).toBeVisible();
    await expect(page.getByRole("radio", { name: "志愿者", exact: true })).toBeChecked();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const guide = await page.locator(".access-guide").boundingBox();
    const form = await page.locator(".access-form-panel").boundingBox();
    expect(guide).not.toBeNull();
    expect(form).not.toBeNull();
    if (viewport.width >= 768) {
      expect(guide!.y).toBe(form!.y);
      expect(guide!.x + guide!.width).toBeCloseTo(form!.x, 0);
      expect(guide!.width / (guide!.width + form!.width)).toBeCloseTo(0.32, 2);
    } else {
      expect(guide!.y).toBeGreaterThanOrEqual(form!.y + form!.height);
      const cards = await page.locator(".access-role-card").evaluateAll((elements) => elements.map((element) => element.getBoundingClientRect().y));
      expect(cards[1]).toBeGreaterThan(cards[0]);
      expect(cards[2]).toBeGreaterThan(cards[1]);
    }
    await expect(page.getByLabel("姓名", { exact: true })).toHaveCSS("font-size", "16px");
    await page.screenshot({ path: testInfo.outputPath(`application-${viewport.width}.png`), fullPage: true });
    await page.getByRole("button", { name: "发送验证邮件" }).click();
    await expect(page.getByLabel("姓名", { exact: true })).toBeFocused();
    await expect(page.getByText("请输入姓名。", { exact: true })).toBeVisible();
    await page.keyboard.type("Layout applicant");
    await page.keyboard.press("Tab");
    await expect(page.getByLabel("邮箱", { exact: true })).toBeFocused();
    await page.keyboard.type("layout@example.invalid");
    await page.keyboard.press("Tab");
    const volunteer = page.getByRole("radio", { name: "志愿者", exact: true });
    await expect(volunteer).toBeFocused();
    await page.keyboard.press("ArrowRight");
    await expect(page.getByRole("radio", { name: "指挥人员", exact: true })).toBeChecked();
    await page.keyboard.press("Tab");
    const submit = page.getByRole("button", { name: "发送验证邮件" });
    await expect(submit).toBeFocused();
    expect(await submit.evaluate((element) => getComputedStyle(element).position)).not.toBe("fixed");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("heading", { name: "申请已提交" })).toBeFocused();
    await expect(page.getByRole("status")).toContainText("如果邮箱可以申请访问");
    expect(submitted).toEqual({ display_name: "Layout applicant", email: "layout@example.invalid", requested_role: "commander" });
    await page.screenshot({ path: testInfo.outputPath(`receipt-${viewport.width}.png`), fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.getByRole("button", { name: "修改申请信息" }).click();
    await expect(page.getByLabel("邮箱", { exact: true })).toHaveValue("layout@example.invalid");
    expect(errors).toEqual([]);
  });

  test(`verification status panel at ${viewport.width}px`, async ({ page }, testInfo) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.setViewportSize(viewport);
    let release!: () => void;
    const pending = new Promise<void>((resolve) => { release = resolve; });
    let requests = 0;
    await page.route("**/api/auth/access-requests/verify", async (route) => {
      requests += 1;
      const { token } = route.request().postDataJSON() as { token: string };
      if (token === "invalid-layout-token") {
        await route.fulfill({ status: 401, json: { error: { code: "unauthorized", message: "验证链接无效或已过期" } } });
        return;
      }
      await pending;
      await route.fulfill({ json: { id: "verified", status: "pending_review", message: "邮箱已验证，申请进入人工审核。" } });
    });
    await page.goto("/#access-verify=layout-token");
    await expect(page.getByRole("heading", { name: "正在验证邮箱" })).toBeVisible();
    await expect(page.getByRole("form")).toHaveCount(0);
    await expect(page).toHaveURL(/\/$/);
    const panel = page.locator(".access-status-panel");
    const initial = await panel.boundingBox();
    expect(initial!.width).toBe(viewport.width < 768 ? viewport.width - 32 : 460);
    expect(initial!.x + initial!.width / 2).toBeCloseTo(viewport.width / 2, 0);
    const headerHeight = viewport.width < 768 ? 64 : 72;
    expect(initial!.y + initial!.height / 2).toBeCloseTo(headerHeight + (viewport.height - headerHeight) / 2, 0);
    await page.screenshot({ path: testInfo.outputPath(`verification-pending-${viewport.width}.png`), fullPage: true });
    release();
    await expect(page.getByRole("heading", { name: "邮箱验证完成" })).toBeVisible();
    await expect(page.getByRole("status")).toContainText("申请进入人工审核");
    expect(requests).toBe(1);
    expect((await panel.boundingBox())!.width).toBe(initial!.width);
    await expect(page.locator("main").getByRole("link", { name: "返回登录" })).toHaveAttribute("href", "/");
    await page.screenshot({ path: testInfo.outputPath(`verification-success-${viewport.width}.png`), fullPage: true });
    await page.goto("/access-request#access-verify=invalid-layout-token");
    await expect(page.getByRole("heading", { name: "验证链接不可用" })).toBeVisible();
    await expect(page.getByRole("status")).toContainText("验证链接无效、已过期或已被使用");
    await expect(page.getByText("invalid-layout-token", { exact: true })).toHaveCount(0);
    await expect(page.getByRole("link", { name: "重新申请" })).toHaveAttribute("href", "/access-request");
    await page.screenshot({ path: testInfo.outputPath(`verification-invalid-${viewport.width}.png`), fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect(errors).toEqual([]);
  });
}

test("short mobile viewport scrolls to submission without a fixed action bar", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 420 });
  await page.goto("/access-request");
  const submit = page.getByRole("button", { name: "发送验证邮件" });
  await submit.scrollIntoViewIfNeeded();
  await expect(submit).toBeInViewport();
  expect(await submit.evaluate((element) => getComputedStyle(element).position)).not.toBe("fixed");
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
