import { expect, test } from "@playwright/test";

const viewports = [
  { width: 1440, height: 900 },
  { width: 1024, height: 900 },
  { width: 768, height: 1024 },
  { width: 390, height: 844 },
  { width: 375, height: 812 },
];

for (const viewport of viewports) {
  test(`login structure and keyboard access at ${viewport.width}px`, async ({ page }, testInfo) => {
    const errors: string[] = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(message.text());
    });
    await page.setViewportSize(viewport);
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "安归 · 统一身份入口" })).toBeVisible();
    await expect(page.getByLabel("邮箱", { exact: true })).toHaveCount(0);

    const entries = page.getByRole("group", { name: "身份入口" }).getByRole("button");
    await expect(entries).toHaveCount(3);
    const mobile = viewport.width < 768;
    const brand = page.getByRole("heading", { name: "安归 · 统一身份入口" }).getByText("安归", { exact: true });
    await expect(brand).toHaveCSS("font-size", mobile ? "32px" : "40px");
    await expect(page.getByText("让求助有人跟进，让行动有序协同", { exact: true })).toHaveCount(0);
    await expect(page.getByText(/连接家属、志愿者与指挥人员/)).toHaveCount(0);
    await expect(page.getByText(/共同帮助失联老人尽早平安回家/)).toHaveCount(0);
    await expect(entries.first()).toContainText(mobile ? "家属 / 知情人" : "志愿者");
    const boxes = await entries.evaluateAll((elements) => elements.map((element) => {
      const { x, y, width, height, bottom } = element.getBoundingClientRect();
      return { x, y, width, height, bottom };
    }));
    for (const box of boxes) {
      expect(box.x).toBeGreaterThanOrEqual(mobile ? 16 : 24);
      expect(box.x + box.width).toBeLessThanOrEqual(viewport.width - (mobile ? 16 : 24) + 1);
      expect(box.height).toBeGreaterThanOrEqual(112);
    }
    if (mobile) {
      expect(boxes[1].y).toBeGreaterThan(boxes[0].bottom);
      expect(boxes[2].y).toBeGreaterThan(boxes[1].bottom);
      expect(boxes[0].width).toBe(viewport.width - 32);
    } else {
      expect(Math.max(...boxes.map((box) => box.width)) - Math.min(...boxes.map((box) => box.width))).toBeLessThan(1);
      expect(new Set(boxes.map((box) => box.y)).size).toBe(1);
      expect(new Set(boxes.map((box) => box.bottom)).size).toBe(1);
      expect(boxes[1].x - boxes[0].x - boxes[0].width).toBeCloseTo(24, 0);
      // Bottom actions remain aligned even when narrower descriptions wrap.
      const actions = await entries.locator(".login-identity-action").evaluateAll((elements) => elements.map((element) => element.getBoundingClientRect().y));
      expect(new Set(actions).size).toBe(1);
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    expect(await page.locator("header").evaluate((element) => element.getBoundingClientRect().height)).toBe(64);
    await page.screenshot({ path: testInfo.outputPath(`identity-${viewport.width}.png`), fullPage: true });

    const familyEntry = page.getByRole("button", { name: /家属 \/ 知情人/ });
    await familyEntry.click();
    const dialog = page.getByRole("dialog", { name: "家属 / 知情人登录" });
    await expect(dialog).toBeVisible();
    await expect(page.getByLabel("邮箱", { exact: true })).not.toBeFocused();
    // React Aria uses native inert in modern browsers and aria-hidden otherwise.
    // Playwright's role selector does not consistently exclude inert subtrees.
    expect(await page.locator(".login-identity-card").first().evaluate((element) => Boolean(element.closest('[inert], [aria-hidden="true"]')))).toBe(true);
    await expect(page.locator(".login-modal-backdrop")).not.toHaveAttribute("data-entering", "true");
    await expect(page.locator(".login-modal-container")).not.toHaveAttribute("data-entering", "true");
    const dialogBox = await dialog.boundingBox();
    expect(dialogBox).not.toBeNull();
    expect(dialogBox!.width).toBe(mobile ? viewport.width : 440);
    expect(await dialog.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(true);
    if (mobile) {
      expect(dialogBox!.x).toBe(0);
      expect(dialogBox!.height).toBeGreaterThanOrEqual(viewport.height);
      expect(await dialog.evaluate((element) => getComputedStyle(element).paddingInlineStart)).toBe("16px");
      expect(await page.getByLabel("邮箱", { exact: true }).evaluate((element) => getComputedStyle(element).fontSize)).toBe("16px");
    }
    await expect(dialog.getByRole("link", { name: "申请访问" })).toHaveAttribute("href", "/access-request?role=family");
    // Walk past both ends of the focus sequence, not just between two controls.
    for (const direction of ["Tab", "Shift+Tab"]) {
      for (let index = 0; index < 10; index += 1) {
        await page.keyboard.press(direction);
        expect(await dialog.evaluate((element) => element.contains(document.activeElement))).toBe(true);
      }
    }
    await page.screenshot({ path: testInfo.outputPath(`authentication-${viewport.width}.png`), fullPage: true });
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    await expect(familyEntry).toBeFocused();
    expect(errors).toEqual([]);
  });
}

test("live resizing changes entry order and header help opens the real instructions", async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 900 });
  await page.goto("/");
  const entries = page.getByRole("group", { name: "身份入口" }).getByRole("button");
  await expect(entries.first()).toContainText("志愿者");
  await page.setViewportSize({ width: 375, height: 812 });
  await expect(entries.first()).toContainText("家属 / 知情人");
  await expect(entries.first().locator("xpath=following-sibling::*[1]")).toContainText("110 报警");
  await page.getByRole("link", { name: "登录帮助", exact: true }).click();
  await expect(page.locator("#login-help")).toHaveAttribute("open", "");
  await expect(page.getByText("请选择身份并申请访问，完成邮箱验证后等待人工审核。收到设置密码邮件后，再返回登录。", { exact: true })).toBeVisible();
  await page.setViewportSize({ width: 768, height: 1024 });
  await expect(entries.first()).toContainText("志愿者");
});

test("short mobile viewport can scroll to validation and submit without a fixed overlay", async ({ page }) => {
  // Reduced viewport height checks scroll reachability, not a real on-screen keyboard.
  await page.setViewportSize({ width: 375, height: 420 });
  await page.goto("/");
  await page.getByRole("button", { name: /家属 \/ 知情人/ }).click();
  const submit = page.getByRole("button", { name: "登录", exact: true });
  await submit.click();
  await expect(page.getByLabel("邮箱", { exact: true })).toBeFocused();
  await expect(page.getByText("请输入邮箱地址。", { exact: true })).toBeVisible();
  await page.getByText("首次使用或忘记密码？", { exact: true }).click();
  await page.getByRole("link", { name: "申请访问" }).scrollIntoViewIfNeeded();
  await expect(page.getByRole("link", { name: "申请访问" })).toBeInViewport();
  expect(await submit.evaluate((element) => getComputedStyle(element).position)).not.toBe("fixed");
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.getByRole("button", { name: "返回身份选择" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
