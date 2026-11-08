import { expect, test, type Page } from "@playwright/test";
import type { AuthUser, GlobalCapability } from "../src/api/auth";

// Isolated, labeled synthetic layout fixtures. Every API request is intercepted;
// these tests do not create real sessions, cases, records or account changes.
const viewports = [
  { width: 1440, height: 900 },
  { width: 1024, height: 900 },
  { width: 768, height: 1024 },
  { width: 390, height: 844 },
];
const roles = ["family", "volunteer", "commander", "admin"] as const;
type Role = typeof roles[number];
const stamp = "2026-11-07T08:00:00Z";

async function mockWorkbench(page: Page, role: Role, options: { failTasks?: boolean; slowCases?: Promise<void>; empty?: boolean; multi?: boolean } = {}) {
  const capabilities: GlobalCapability[] = options.multi ? ["commander", "volunteer", "admin"] : role === "family" ? [] : [role];
  const user: AuthUser = { id: "synthetic-layout-user", display_name: "布局验收专用合成账号", email: "dashboard-layout@example.invalid", account_type: "member", global_capabilities: capabilities };
  const requests: string[] = [];
  const unexpected: string[] = [];
  let tasksUnavailable = Boolean(options.failTasks);
  const accessRole = role === "admin" ? "volunteer" : role;
  const cases = options.empty || role === "admin" ? [] : [0, 1, 2].map((index) => ({ id: `synthetic-case-${index}`, case_code: `示例案件 ${index + 1}`, display_name: "布局验收合成记录", access_role: accessRole, status: "active", last_seen_at: stamp, last_seen_location: "合成测试区域", created_at: stamp, updated_at: stamp }));
  await page.addInitScript(() => sessionStorage.setItem("angui.session.token", "isolated-dashboard-layout-token"));
  await page.route((url) => url.pathname.startsWith("/api/"), async (route) => {
    const path = new URL(route.request().url()).pathname.replace(/^\/api/, "");
    requests.push(path);
    if (path === "/auth/me") return route.fulfill({ json: user });
    if (path === "/health") return route.fulfill({ json: { status: "ok", service: "synthetic-layout-service", version: "layout-fixture" } });
    if (path === "/cases") { if (options.slowCases) await options.slowCases; return route.fulfill({ json: cases }); }
    if (path === "/cases/command-intake") return route.fulfill({ json: options.empty ? [] : [{ id: "synthetic-unaccepted", case_code: "示例待受理案件", area_hint: "合成测试范围", last_seen_at: stamp, elder_age: 70, created_at: stamp }] });
    if (cases.some((item) => path === `/cases/${item.id}`)) return route.fulfill({ json: { ...cases.find((item) => path === `/cases/${item.id}`), clues: [{ status: "pending_review" }] } });
    if (path === "/intake-sessions") return route.fulfill({ json: options.empty ? [] : [{ id: "synthetic-draft", status: "collecting", phase: "phase_one", missing_fields: ["basic_information"], next_question: null, updated_at: stamp }] });
    if (path === "/tasks/mine") {
      if (tasksUnavailable) return route.fulfill({ status: 503, json: { error: { code: "unavailable", message: "synthetic-private-detail" } } });
      return route.fulfill({ json: options.empty ? [] : ["active", "assigned", "completed"].map((status, index) => ({ id: `synthetic-task-${index}`, case_id: "synthetic-case-0", title: `示例任务 ${index + 1}：核对授权区域内的工作信息`, status, updated_at: stamp, due_at: stamp, collaborators: [] })) });
    }
    if (path === "/admin/archive-drafts") return route.fulfill({ json: [] });
    if (path === "/admin/access-requests") return route.fulfill({ json: options.empty ? [] : [{ id: "synthetic-request", status: "pending_review", requested_role: "volunteer", created_at: stamp }] });
    if (path === "/users/me/profile") return route.fulfill({ json: { ...user, team_name: null, avatar_reference: null, preferences: { locale: "zh-CN", reduced_motion: false } } });
    unexpected.push(path);
    return route.fulfill({ status: 404, json: { error: { code: "not_found", message: "Unconfigured synthetic fixture" } } });
  });
  return { requests, unexpected, recoverTasks: () => { tasksUnavailable = false; } };
}

for (const role of roles) {
  for (const viewport of viewports) {
    test(`${role} workbench hierarchy and navigation at ${viewport.width}px`, async ({ page }, testInfo) => {
      await page.setViewportSize(viewport);
      const { requests, unexpected } = await mockWorkbench(page, role);
      const errors: string[] = [];
      page.on("pageerror", (error) => errors.push(error.message));
      page.on("console", (message) => { if (message.type() === "error") errors.push(message.text()); });
      await page.goto("/");
      await expect(page.getByRole("heading", { level: 1, name: "总览首页" })).toBeVisible();
      await expect(page.getByText("正在加载工作待办")).toHaveCount(0);
      await expect(page.locator(".app-shell-header")).toHaveCSS("height", "64px");
      const nav = page.getByRole("navigation", { name: "主要导航" });
      const navLinks = nav.getByRole("link").filter({ visible: true });
      const cards = page.locator(".dashboard-entry");
      const boxes = await cards.evaluateAll((items) => items.map((item) => { const b = item.getBoundingClientRect(); return { x: b.x, y: b.y, width: b.width, height: b.height, right: b.right, bottom: b.bottom }; }));
      expect(boxes).toHaveLength(role === "commander" ? 2 : role === "volunteer" ? 3 : 4);
      const columns = viewport.width >= 1280 ? 4 : viewport.width >= 768 ? 2 : 1;
      const firstRow = boxes.slice(0, Math.min(columns, boxes.length));
      expect(new Set(firstRow.map((b) => b.y)).size).toBe(1);
      expect(new Set(firstRow.map((b) => b.height)).size).toBe(1);
      for (const box of boxes) { expect(box.x).toBeGreaterThanOrEqual(viewport.width >= 1024 ? 176 + 24 : 16); expect(box.right).toBeLessThanOrEqual(viewport.width - 16); }
      if (viewport.width >= 1024) {
        await expect(page.locator(".app-shell-sidebar")).toHaveCSS("width", viewport.width >= 1200 ? "208px" : "176px");
        await expect(nav).toHaveCSS("display", "flex");
      } else {
        expect(await navLinks.count()).toBeLessThanOrEqual(4);
        const navBox = (await nav.boundingBox())!;
        expect(navBox.y).toBeGreaterThan(0);
        expect(navBox.y + navBox.height).toBeLessThanOrEqual(viewport.height);
        for (const link of await navLinks.all()) { const b = (await link.boundingBox())!; expect(b.height).toBeGreaterThanOrEqual(44); expect(b.width).toBeGreaterThanOrEqual(44); }
      }
      if (viewport.width === 390) {
        const firstAction = (await cards.first().locator(".dashboard-entry-action").boundingBox())!;
        expect(firstAction.y + firstAction.height).toBeLessThan(viewport.height - 80);
        for (let index = 1; index < boxes.length; index++) expect(boxes[index].y).toBeGreaterThanOrEqual(boxes[index - 1].bottom + 11);
      }
      if (viewport.width === 1440) {
        const work = await page.locator(".dashboard-work-grid > *").evaluateAll((items) => items.map((item) => { const b = item.getBoundingClientRect(); return { y: b.y, width: b.width }; }));
        expect(work[0].y).toBe(work[1].y); expect(work[0].width).toBeGreaterThan(work[1].width);
      }
      const primary = { family: "发起求助", volunteer: "我的任务", commander: "案件与线索", admin: "账号审核" }[role];
      await expect(cards.first()).toContainText(primary);
      if (role === "admin") { expect(requests).not.toContain("/cases/command-intake"); expect(requests).not.toContain("/tasks/mine"); await expect(page.getByRole("link", { name: /发起求助|公开进展|案件与线索/ })).toHaveCount(0); }
      if (role !== "commander") expect(requests).not.toContain("/cases/command-intake");
      if (role !== "volunteer") expect(requests).not.toContain("/tasks/mine");
      if (role !== "family") expect(requests).not.toContain("/intake-sessions");
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      await page.screenshot({ path: testInfo.outputPath(`${role}-${viewport.width}.png`), fullPage: true });
      await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
      expect((await page.locator(".app-shell-header").boundingBox())!.y).toBe(0);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      const lastContent = role === "admin" ? page.locator(".dashboard-archive") : page.getByRole("region", { name: "最近更新" });
      if (viewport.width < 1024) {
        const lastBox = (await lastContent.boundingBox())!;
        expect(lastBox.y + lastBox.height).toBeLessThanOrEqual((await page.locator(".app-shell-sidebar").boundingBox())!.y);
      }
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.keyboard.press("Tab");
      await expect(page.getByRole("link", { name: "跳转到工作区内容" })).toBeFocused();
      await page.keyboard.press("Enter");
      await expect(page.locator("#workspace-content")).toBeFocused();
      // Drive an existing destination through the actual router and a mocked boundary.
      await page.getByRole("link", { name: "个人账户", exact: true }).click();
      await expect(page.getByRole("heading", { name: "个人资料" })).toBeVisible();
      expect(unexpected).toEqual([]);
      expect(errors).toEqual([]);
    });
  }
}

test("partial failure is distinct from zero and can be refreshed", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const { unexpected, recoverTasks } = await mockWorkbench(page, "volunteer", { failTasks: true });
  await page.goto("/");
  const row = page.getByRole("link", { name: /待处理任务/ });
  await expect(row).toContainText("—");
  await expect(page.getByText("synthetic-private-detail")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "我的任务", exact: true })).toBeVisible();
  recoverTasks();
  await page.getByRole("button", { name: "刷新数据" }).click();
  await expect(row.getByLabel("2 项")).toBeVisible();
  expect(unexpected).toEqual([]);
});

test("multi-capability mobile navigation stays capped and all authorized entries remain available", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await mockWorkbench(page, "commander", { multi: true, empty: true });
  await page.goto("/");
  await expect(page.getByText("正在加载工作待办")).toHaveCount(0);
  await expect(page.locator(".dashboard-entry")).toHaveCount(7);
  await expect(page.getByRole("navigation", { name: "主要导航" }).getByRole("link").filter({ visible: true })).toHaveCount(4);
  await page.getByRole("link", { name: /归档审核.*进入/ }).click();
  await expect(page.getByRole("heading", { name: "案例整理草稿" })).toBeInViewport();
  expect((await page.getByRole("heading", { name: "案例整理草稿" }).boundingBox())!.y).toBeGreaterThanOrEqual(64);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test("initial loading and empty data retain the primary family action", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  let release!: () => void;
  const slowCases = new Promise<void>((resolve) => { release = resolve; });
  await mockWorkbench(page, "family", { slowCases, empty: true });
  await page.goto("/");
  await expect(page.getByText("正在加载工作待办")).toBeVisible();
  await expect(page.getByRole("link", { name: /发起求助/ })).toBeInViewport();
  await expect(page.getByRole("button", { name: "刷新数据" })).toBeDisabled();
  release();
  await expect(page.getByRole("link", { name: /继续求助登记/ }).getByLabel("0 项")).toBeVisible();
  await expect(page.getByText("暂无最近更新")).toBeVisible();
});
