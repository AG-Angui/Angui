import { expect, test } from "@playwright/test";

const viewports = [
  { width: 390, height: 844 },
  { width: 768, height: 1024 },
  { width: 1024, height: 900 },
  { width: 1440, height: 900 },
];
// Isolated synthetic values; every password-setup request below is intercepted.
const syntheticPassword = "isolated-layout-example";

for (const viewport of viewports) {
  test(`password form alignment, keyboard and field errors at ${viewport.width}px`, async ({ page }, testInfo) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
    await page.setViewportSize(viewport);
    let requests = 0;
    await page.route("**/api/auth/password-setup", async (route) => {
      requests += 1;
      await route.fulfill({ status: 204 });
    });
    await page.goto("/#password-setup=isolated-layout-token");
    const password = page.getByLabel("新密码", { exact: true });
    const confirm = page.getByLabel("确认密码", { exact: true });
    const submit = page.getByRole("button", { name: "保存密码" });
    const panel = page.locator(".password-setup-panel");
    await expect(page.getByRole("heading", { level: 1, name: "设置登录密码" })).toBeVisible();
    await expect(page.locator(".password-setup-header")).toHaveCSS("height", "64px");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const card = (await panel.boundingBox())!;
    expect(card.width).toBe(viewport.width < 768 ? viewport.width - 32 : 460);
    expect(card.x + card.width / 2).toBeCloseTo(viewport.width / 2, 0);
    if (viewport.width >= 768) {
      expect(card.y + card.height / 2).toBeCloseTo(64 + (viewport.height - 64) / 2, 0);
      await expect(panel).toHaveCSS("padding", "32px");
    } else {
      expect(card.y).toBe(96);
      await expect(panel).toHaveCSS("border-top-width", "0px");
    }
    const boxes = await Promise.all([password, confirm, submit].map((element) => element.boundingBox()));
    for (const box of boxes) {
      expect(box!.x).toBe(boxes[0]!.x);
      expect(box!.width).toBe(boxes[0]!.width);
      expect(box!.height).toBeGreaterThanOrEqual(48);
    }
    for (const input of [password, confirm]) {
      await expect(input).toHaveCSS("font-size", "16px");
      expect(await input.evaluate((element) => parseFloat(getComputedStyle(element).paddingRight))).toBeGreaterThanOrEqual(48);
    }
    const reveal = page.getByRole("button", { name: "显示新密码" });
    const revealBox = (await reveal.boundingBox())!;
    expect(revealBox.x).toBeGreaterThan(boxes[0]!.x);
    expect(revealBox.x + revealBox.width).toBeLessThanOrEqual(boxes[0]!.x + boxes[0]!.width);
    expect(revealBox.height).toBeGreaterThanOrEqual(44);
    await page.screenshot({ path: testInfo.outputPath(`form-${viewport.width}.png`), fullPage: true });

    await submit.click();
    await expect(password).toBeFocused();
    await expect(page.getByText("请输入新密码。", { exact: true })).toBeVisible();
    expect(requests).toBe(0);
    await page.keyboard.type("short");
    await submit.click();
    await expect(password).toBeFocused();
    await expect(page.getByText("密码长度须为 12–256 个字符。", { exact: true })).toBeVisible();
    await password.fill(syntheticPassword);
    await page.keyboard.press("Tab");
    await expect(reveal).toBeFocused();
    await page.keyboard.press("Space");
    await expect(password).toHaveAttribute("type", "text");
    await page.keyboard.press("Space");
    await expect(password).toHaveAttribute("type", "password");
    await page.keyboard.press("Tab");
    await expect(confirm).toBeFocused();
    await page.keyboard.type("different-example-password");
    await submit.click();
    await expect(confirm).toBeFocused();
    const fieldError = page.getByText("两次输入的密码不一致。", { exact: true });
    await expect(fieldError).toBeVisible();
    expect((await fieldError.boundingBox())!.y).toBeGreaterThanOrEqual((await confirm.boundingBox())!.y + (await confirm.boundingBox())!.height);
    await page.screenshot({ path: testInfo.outputPath(`field-errors-${viewport.width}.png`), fullPage: true });
    await confirm.fill(syntheticPassword);
    await page.keyboard.press("Tab");
    await expect(page.getByRole("button", { name: "显示确认密码" })).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(submit).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("heading", { name: "密码设置成功" })).toBeFocused();
    expect(requests).toBe(1);
    expect(errors).toEqual([]);
  });

  test(`password pending, success and expired-link panels at ${viewport.width}px`, async ({ page }, testInfo) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.setViewportSize(viewport);
    let release!: () => void;
    const pending = new Promise<void>((resolve) => { release = resolve; });
    let requests = 0;
    await page.route("**/api/auth/password-setup", async (route) => {
      requests += 1;
      const payload = route.request().postDataJSON() as { token: string; password: string };
      expect(Object.keys(payload).sort()).toEqual(["password", "token"]);
      expect(payload.password).toBe(syntheticPassword);
      expect(route.request().headers().authorization).toBeUndefined();
      if (payload.token === "isolated-expired-token") {
        await route.fulfill({ status: 401, json: { error: { code: "unauthorized", message: "密码设置链接无效或已过期" } } });
        return;
      }
      expect(payload.token).toBe("isolated-layout-token");
      await pending;
      await route.fulfill({ status: 204 });
    });
    await page.goto("/#password-setup=isolated-layout-token");
    const panel = page.locator(".password-setup-panel");
    await expect(page.getByLabel("新密码", { exact: true })).toBeVisible();
    const initial = (await panel.boundingBox())!;
    await page.getByLabel("新密码", { exact: true }).fill(syntheticPassword);
    await page.getByLabel("确认密码", { exact: true }).fill(syntheticPassword);
    await page.getByRole("button", { name: "保存密码" }).click();
    await expect(page.getByRole("heading", { name: "正在设置密码" })).toBeVisible();
    await expect(page.getByRole("form")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "正在保存…" })).toBeDisabled();
    await expect(panel).toHaveAttribute("aria-busy", "true");
    expect((await panel.boundingBox())!.width).toBe(initial.width);
    if (viewport.width >= 768) expect((await panel.boundingBox())!.height).toBe(initial.height);
    await page.screenshot({ path: testInfo.outputPath(`pending-${viewport.width}.png`), fullPage: true });
    release();
    await expect(page.getByRole("heading", { name: "密码设置成功" })).toBeFocused();
    await expect(page.getByRole("form")).toHaveCount(0);
    expect((await panel.boundingBox())!.width).toBe(initial.width);
    await expect(page.locator("main").getByRole("link", { name: "返回登录" })).toHaveAttribute("href", "/");
    await page.screenshot({ path: testInfo.outputPath(`success-${viewport.width}.png`), fullPage: true });
    expect(requests).toBe(1);

    // A new email link is a fresh document load, not an in-page fragment change.
    await page.goto("/?layout=expired#password-setup=isolated-expired-token");
    await page.getByLabel("新密码", { exact: true }).fill(syntheticPassword);
    await page.getByLabel("确认密码", { exact: true }).fill(syntheticPassword);
    await page.getByRole("button", { name: "保存密码" }).click();
    await expect(page.getByRole("heading", { name: "密码设置链接不可用" })).toBeFocused();
    await expect(page.getByRole("status")).toContainText("联系组织管理员");
    await expect(page.getByRole("form")).toHaveCount(0);
    await expect(page.getByText("isolated-expired-token", { exact: true })).toHaveCount(0);
    expect((await panel.boundingBox())!.width).toBe(initial.width);
    await page.screenshot({ path: testInfo.outputPath(`expired-${viewport.width}.png`), fullPage: true });
    await page.locator("main").getByRole("link", { name: "返回登录" }).click();
    await expect(page.getByRole("heading", { name: "安归 · 统一身份入口" })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect(errors).toEqual([]);
  });
}

test("mobile reduced-height viewport keeps focused inputs and the in-flow action reachable", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.route("**/api/auth/password-setup", (route) => route.fulfill({ status: 204 }));
  await page.goto("/#password-setup=isolated-layout-token");
  await page.getByLabel("新密码", { exact: true }).fill(syntheticPassword);
  const confirm = page.getByLabel("确认密码", { exact: true });
  await confirm.focus();
  // Desktop Chromium cannot open a real OS keyboard. This only models reduced viewport height.
  await page.setViewportSize({ width: 390, height: 420 });
  await confirm.scrollIntoViewIfNeeded();
  await expect(confirm).toBeFocused();
  await expect(confirm).toBeInViewport();
  await confirm.fill(syntheticPassword);
  const submit = page.getByRole("button", { name: "保存密码" });
  await submit.scrollIntoViewIfNeeded();
  await expect(submit).toBeInViewport();
  expect(await submit.evaluate((element) => getComputedStyle(element).position)).not.toBe("fixed");
  expect(await page.evaluate(() => document.documentElement.scrollHeight > innerHeight)).toBe(true);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: testInfo.outputPath("reduced-height-mobile.png"), fullPage: true });
  await submit.click();
  await expect(page.getByRole("heading", { name: "密码设置成功" })).toBeVisible();
});

test("temporary failure preserves the mobile form and offers an explicit retry without exposing server details", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });
  let requests = 0;
  await page.route("**/api/auth/password-setup", async (route) => {
    requests += 1;
    if (requests === 1) {
      await route.fulfill({ status: 503, json: { error: { code: "unavailable", message: "private-server-detail" } } });
    } else await route.fulfill({ status: 204 });
  });
  await page.goto("/#password-setup=isolated-layout-token");
  await page.getByLabel("新密码", { exact: true }).fill(syntheticPassword);
  await page.getByLabel("确认密码", { exact: true }).fill(syntheticPassword);
  await page.getByRole("button", { name: "保存密码" }).click();
  await expect(page.getByRole("alert")).toBeFocused();
  await expect(page.getByRole("alert")).toContainText("服务暂时不可用");
  await expect(page.getByText("private-server-detail")).toHaveCount(0);
  await expect(page.getByLabel("新密码", { exact: true })).toHaveValue(syntheticPassword);
  await expect(page.getByLabel("确认密码", { exact: true })).toHaveValue(syntheticPassword);
  await expect(page.getByLabel("新密码", { exact: true })).toHaveAttribute("type", "password");
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await page.screenshot({ path: testInfo.outputPath("service-error-mobile.png"), fullPage: true });
  await page.getByRole("button", { name: "保存密码" }).click();
  await expect(page.getByRole("heading", { name: "密码设置成功" })).toBeVisible();
  expect(requests).toBe(2);
});

for (const width of [320, 375]) {
  test(`password setup has no horizontal overflow at narrow ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 667 });
    await page.goto("/#password-setup=");
    await expect(page.getByRole("heading", { name: "密码设置链接不可用" })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.goto("/?layout=form#password-setup=isolated-layout-token");
    await expect(page.getByLabel("新密码", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "保存密码" }).scrollIntoViewIfNeeded();
    await expect(page.getByRole("button", { name: "保存密码" })).toBeInViewport();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  });
}
