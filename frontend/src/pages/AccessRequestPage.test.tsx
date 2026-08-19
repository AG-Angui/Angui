import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AccessRequestPage } from "./AccessRequestPage";

const mocked = vi.hoisted(() => ({
  createAccessRequest: vi.fn(),
  verifyAccessRequest: vi.fn(),
}));

vi.mock("../api/accessRequests", () => ({
  createAccessRequest: (...args: unknown[]) => mocked.createAccessRequest(...args),
  verifyAccessRequest: (...args: unknown[]) => mocked.verifyAccessRequest(...args),
}));

describe("AccessRequestPage", () => {
  beforeEach(() => {
    window.history.replaceState({}, "", "/access-request");
    mocked.createAccessRequest.mockReset();
    mocked.verifyAccessRequest.mockReset();
  });

  afterEach(() => {
    window.history.replaceState({}, "", "/");
  });

  it("submits the selected role and displays the generic email-delivery response", async () => {
    mocked.createAccessRequest.mockResolvedValue({
      id: "request-1",
      status: "pending_verification",
      message: "如果邮箱可以申请访问，我们会发送一封验证邮件，请查收后继续。",
    });
    render(<AccessRequestPage />);

    fireEvent.change(screen.getByLabelText("姓名"), { target: { value: "测试申请人" } });
    fireEvent.change(screen.getByLabelText("邮箱"), { target: { value: "applicant@example.invalid" } });
    fireEvent.change(screen.getByLabelText("期望身份"), { target: { value: "volunteer" } });
    fireEvent.click(screen.getByRole("button", { name: "发送验证邮件" }));

    await waitFor(() => expect(mocked.createAccessRequest).toHaveBeenCalledWith({
      display_name: "测试申请人",
      email: "applicant@example.invalid",
      requested_role: "volunteer",
    }));
    expect(await screen.findByRole("status")).toHaveTextContent("如果邮箱可以申请访问");
    expect(screen.queryByLabelText("验证令牌")).not.toBeInTheDocument();
  });

  it("consumes an email verification token from the URL fragment and removes it from the address bar", async () => {
    mocked.verifyAccessRequest.mockResolvedValue({
      id: "request-1",
      status: "pending_review",
      message: "邮箱已验证，申请进入人工审核。",
    });
    window.history.replaceState({}, "", "/#access-verify=one-time-token");

    render(<AccessRequestPage />);

    await waitFor(() => expect(mocked.verifyAccessRequest).toHaveBeenCalledWith("one-time-token"));
    expect(await screen.findByRole("status")).toHaveTextContent("邮箱已验证，申请进入人工审核。");
    expect(window.location.hash).toBe("");
  });

  it("does not expose a raw verification token when the link is invalid", async () => {
    mocked.verifyAccessRequest.mockRejectedValue(new Error("verification link is invalid or expired"));
    window.history.replaceState({}, "", "/#access-verify=secret-token-must-not-render");

    render(<AccessRequestPage />);

    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "验证链接无效、已过期或已被使用",
      ),
    );
    expect(screen.queryByText("secret-token-must-not-render")).not.toBeInTheDocument();
  });
});
