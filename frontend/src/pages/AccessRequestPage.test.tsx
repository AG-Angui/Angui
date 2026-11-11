import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { StrictMode } from "react";
import { ApiClientError } from "../api/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AccessRequestPage } from "./AccessRequestPage";

const mocked = vi.hoisted(() => ({
  createAccessRequest: vi.fn(),
  verifyAccessRequest: vi.fn(),
}));

vi.mock("../api/accessRequests", () => ({
  createAccessRequest: (...args: unknown[]) =>
    mocked.createAccessRequest(...args),
  verifyAccessRequest: (...args: unknown[]) =>
    mocked.verifyAccessRequest(...args),
}));

function fillForm() {
  fireEvent.change(screen.getByLabelText("姓名"), { target: { value: " 测试申请人 " } });
  fireEvent.change(screen.getByLabelText("邮箱"), { target: { value: " applicant@example.invalid " } });
}

const verifiedResponse = {
  id: "request-1",
  status: "pending_review",
  message: "邮箱已验证，申请进入人工审核。",
};

describe("AccessRequestPage", () => {
  beforeEach(() => {
    window.history.replaceState({}, "", "/access-request");
    mocked.createAccessRequest.mockReset();
    mocked.verifyAccessRequest.mockReset();
  });

  afterEach(() => {
    window.history.replaceState({}, "", "/");
  });

  it.each([
    ["family", "家属 / 知情人"],
    ["volunteer", "志愿者"],
    ["commander", "指挥人员"],
    ["admin", "家属 / 知情人"],
  ])("preselects supported query roles only: %s", (role, label) => {
    window.history.replaceState({}, "", `/access-request?role=${role}`);
    render(<AccessRequestPage />);
    expect(screen.getByRole("radio", { name: label })).toBeChecked();
    expect(screen.getAllByRole("radio")).toHaveLength(3);
    expect(screen.getByRole("group", { name: /期望身份/ })).toBeInTheDocument();
  });

  it("validates required fields and focuses the first error without calling the API", () => {
    render(<AccessRequestPage />);
    fireEvent.click(screen.getByRole("button", { name: "发送验证邮件" }));
    expect(screen.getByLabelText("姓名")).toHaveFocus();
    expect(screen.getByText("请输入姓名。")).toHaveAttribute("role", "alert");
    expect(screen.getByLabelText("邮箱")).toHaveAttribute("aria-invalid", "true");
    fireEvent.change(screen.getByLabelText("姓名"), { target: { value: "申请人" } });
    fireEvent.change(screen.getByLabelText("邮箱"), { target: { value: "not-an-email" } });
    fireEvent.click(screen.getByRole("button", { name: "发送验证邮件" }));
    expect(screen.getByLabelText("邮箱")).toHaveFocus();
    expect(screen.getByText("请输入有效的邮箱地址。")).toBeInTheDocument();
    expect(mocked.createAccessRequest).not.toHaveBeenCalled();
  });

  it("rejects an overlong name", () => {
    render(<AccessRequestPage />);
    fillForm();
    fireEvent.change(screen.getByLabelText("姓名"), { target: { value: "名".repeat(121) } });
    fireEvent.submit(screen.getByRole("form", { name: "访问申请" }));
    expect(screen.getByText("姓名不能超过 120 个字符。")).toBeInTheDocument();
    expect(mocked.createAccessRequest).not.toHaveBeenCalled();
  });

  it("guards duplicate submits and disables fields while pending, then focuses the receipt", async () => {
    let finish!: (value: unknown) => void;
    mocked.createAccessRequest.mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
    render(<AccessRequestPage />);
    fillForm();
    const form = screen.getByRole("form", { name: "访问申请" });
    fireEvent.submit(form);
    fireEvent.submit(form);
    expect(mocked.createAccessRequest).toHaveBeenCalledTimes(1);
    expect(mocked.createAccessRequest).toHaveBeenCalledWith({ display_name: "测试申请人", email: "applicant@example.invalid", requested_role: "family" });
    expect(form).toHaveAttribute("aria-busy", "true");
    expect(screen.getByLabelText("姓名")).toBeDisabled();
    expect(screen.getByRole("radio", { name: "志愿者" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "正在提交…" })).toBeDisabled();
    await act(async () => { finish({ id: "request-1", status: "pending_verification", message: "如果邮箱可以申请访问，我们会发送一封验证邮件，请查收后继续。" }); });
    expect(screen.getByRole("heading", { name: "申请已提交" })).toHaveFocus();
    expect(screen.queryByText("邮件发送成功")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "修改申请信息" }));
    expect(screen.getByLabelText("姓名")).toHaveValue(" 测试申请人 ");
    expect(mocked.createAccessRequest).toHaveBeenCalledTimes(1);
  });

  it("preserves form data on failure and clears the safe server error when editing", async () => {
    mocked.createAccessRequest.mockRejectedValue(new ApiClientError(0, "network_error", "网络连接失败，请检查服务连接后重试。", "private server detail"));
    render(<AccessRequestPage />);
    fillForm();
    fireEvent.click(screen.getByRole("button", { name: "发送验证邮件" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("网络连接失败");
    expect(screen.getByLabelText("姓名")).toHaveValue(" 测试申请人 ");
    expect(screen.getByRole("button", { name: "发送验证邮件" })).toBeEnabled();
    expect(screen.queryByText("private server detail")).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("姓名"), { target: { value: "修改后的姓名" } });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shares one verification request across StrictMode effect replay and shows the eventual result", async () => {
    let finish!: (value: unknown) => void;
    mocked.verifyAccessRequest.mockImplementation(() => new Promise((resolve) => { finish = resolve; }));
    window.history.replaceState({ kept: true }, "", "/access-request?role=volunteer#access-verify=strict-token");
    render(<StrictMode><AccessRequestPage /></StrictMode>);
    expect(screen.getByRole("heading", { name: "正在验证邮箱" })).toBeInTheDocument();
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
    expect(mocked.verifyAccessRequest).toHaveBeenCalledTimes(1);
    expect(window.location.hash).toBe("");
    expect(window.location.search).toBe("?role=volunteer");
    expect(window.history.state).toEqual({ kept: true });
    await act(async () => { finish(verifiedResponse); });
    expect(screen.getByRole("heading", { name: "邮箱验证完成" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("申请进入人工审核");
    expect(screen.queryByText("strict-token")).not.toBeInTheDocument();
  });

  it("keeps empty verification fragments in the invalid status panel, not the application form", () => {
    window.history.replaceState({}, "", "/#access-verify=");
    render(<AccessRequestPage />);
    expect(screen.getByRole("heading", { name: "验证链接不可用" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "重新申请" })).toHaveAttribute("href", "/access-request");
    expect(mocked.verifyAccessRequest).not.toHaveBeenCalled();
    expect(screen.queryByLabelText("姓名")).not.toBeInTheDocument();
  });

  it.each([0, 503])("offers explicit verification retry for transient errors (%i)", async (status) => {
    mocked.verifyAccessRequest.mockRejectedValueOnce(new ApiClientError(status, "unavailable", "unsafe generic detail")).mockResolvedValueOnce(verifiedResponse);
    window.history.replaceState({}, "", "/#access-verify=retry-token");
    render(<AccessRequestPage />);
    fireEvent.click(await screen.findByRole("button", { name: "重试验证" }));
    expect(await screen.findByRole("heading", { name: "邮箱验证完成" })).toBeInTheDocument();
    expect(mocked.verifyAccessRequest).toHaveBeenCalledTimes(2);
    expect(mocked.verifyAccessRequest).toHaveBeenLastCalledWith("retry-token");
    expect(window.location.hash).toBe("");
  });

  it("uses link-specific text for unauthorized verification rather than the login error", async () => {
    mocked.verifyAccessRequest.mockRejectedValue(new ApiClientError(401, "unauthorized", "邮箱或密码错误，请重新输入。"));
    window.history.replaceState({}, "", "/#access-verify=expired");
    render(<AccessRequestPage />);
    expect(await screen.findByRole("heading", { name: "验证链接不可用" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("验证链接无效、已过期或已被使用");
    expect(screen.queryByText(/邮箱或密码错误/)).not.toBeInTheDocument();
  });

  it("submits the selected role and displays the generic email-delivery response", async () => {
    mocked.createAccessRequest.mockResolvedValue({
      id: "request-1",
      status: "pending_verification",
      message: "如果邮箱可以申请访问，我们会发送一封验证邮件，请查收后继续。",
    });
    render(<AccessRequestPage />);

    fireEvent.change(screen.getByLabelText("姓名"), {
      target: { value: "测试申请人" },
    });
    fireEvent.change(screen.getByLabelText("邮箱"), {
      target: { value: "applicant@example.invalid" },
    });
    fireEvent.click(screen.getByRole("radio", { name: "志愿者" }));
    fireEvent.click(screen.getByRole("button", { name: "发送验证邮件" }));

    await waitFor(() =>
      expect(mocked.createAccessRequest).toHaveBeenCalledWith({
        display_name: "测试申请人",
        email: "applicant@example.invalid",
        requested_role: "volunteer",
      }),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      "如果邮箱可以申请访问",
    );
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

    await waitFor(() =>
      expect(mocked.verifyAccessRequest).toHaveBeenCalledWith("one-time-token"),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      "邮箱已验证，申请进入人工审核。",
    );
    expect(window.location.hash).toBe("");
  });

  it("does not expose a raw verification token when the link is invalid", async () => {
    mocked.verifyAccessRequest.mockRejectedValue(
      new Error("verification link is invalid or expired"),
    );
    window.history.replaceState(
      {},
      "",
      "/#access-verify=secret-token-must-not-render",
    );

    render(<AccessRequestPage />);

    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent(
        "验证链接无效、已过期或已被使用",
      ),
    );
    expect(
      screen.queryByText("secret-token-must-not-render"),
    ).not.toBeInTheDocument();
  });
});
