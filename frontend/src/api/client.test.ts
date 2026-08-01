import { describe, expect, it, vi } from "vitest";
import { SESSION_EXPIRED_EVENT, apiRequest } from "./client";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

describe("apiRequest", () => {
  it.each([
    [400, "validation_error", "提交的信息不符合要求，请检查后重试。"],
    [403, "forbidden", "你没有执行此操作的权限。"],
    [404, "not_found", "未找到可访问的资源，它可能已不存在或你没有访问权限。"],
    [409, "conflict", "当前数据状态已变化，请刷新后再试。"],
  ])(
    "maps documented HTTP %i errors to user-safe messages",
    async (status, code, message) => {
      vi.stubGlobal(
        "fetch",
        vi.fn().mockResolvedValue(
          jsonResponse(status, {
            error: { code, message: "internal state detail" },
          }),
        ),
      );

      await expect(apiRequest("/cases/example")).rejects.toMatchObject({
        status,
        code,
        message,
      });
    },
  );

  it("keeps invalid-login messaging separate from an expired session", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          jsonResponse(401, { error: { code: "unauthorized" } }),
        ),
    );

    await expect(apiRequest("/auth/login")).rejects.toMatchObject({
      status: 401,
      code: "unauthorized",
      message: "邮箱或密码错误，请重新输入。",
    });
  });

  it("clears an authenticated session signal when the server returns 401", async () => {
    const expired = vi.fn();
    window.addEventListener(SESSION_EXPIRED_EVENT, expired);
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(
          jsonResponse(401, { error: { code: "unauthorized" } }),
        ),
    );

    await expect(
      apiRequest("/cases", {}, "test-session"),
    ).rejects.toMatchObject({
      status: 401,
      code: "unauthorized",
    });
    expect(expired).toHaveBeenCalledOnce();
    window.removeEventListener(SESSION_EXPIRED_EVENT, expired);
  });

  it("reports network failures without exposing the transport error", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockRejectedValue(new TypeError("socket details")),
    );

    await expect(apiRequest("/health")).rejects.toMatchObject({
      status: 0,
      code: "network_error",
      message: "网络连接失败，请检查服务连接后重试。",
    });
  });
});
