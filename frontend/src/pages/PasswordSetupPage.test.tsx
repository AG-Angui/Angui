import { act, fireEvent, render, screen } from "@testing-library/react";
import { StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClientError } from "../api/client";
import { PasswordSetupPage } from "./PasswordSetupPage";

const mocked = vi.hoisted(() => ({ setPassword: vi.fn() }));
vi.mock("../api/accessRequests", () => ({ setPassword: mocked.setPassword }));

const syntheticPassword = "isolated-layout-example";

function fillPasswords(password = syntheticPassword, confirm = password) {
  fireEvent.change(screen.getByLabelText("新密码", { exact: true }), { target: { value: password } });
  fireEvent.change(screen.getByLabelText("确认密码", { exact: true }), { target: { value: confirm } });
}

function save() {
  fireEvent.click(screen.getByRole("button", { name: "保存密码" }));
}

beforeEach(() => {
  window.history.replaceState({}, "", "/#password-setup=isolated-layout-token");
  mocked.setPassword.mockReset();
});
afterEach(() => window.history.replaceState({}, "", "/"));

describe("PasswordSetupPage", () => {
  it("shows one task, two labelled inputs and actual password requirements without making a preflight request", () => {
    render(<StrictMode><PasswordSetupPage /></StrictMode>);
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(screen.getByRole("heading", { name: "设置登录密码" })).toBeInTheDocument();
    for (const label of ["新密码", "确认密码"]) {
      const input = screen.getByLabelText(label, { exact: true });
      expect(input).toHaveAttribute("type", "password");
      expect(input).toHaveAttribute("autocomplete", "new-password");
      expect(input).toBeRequired();
    }
    expect(screen.getByText("长度为 12–256 个字符，两次输入须一致。")).toBeInTheDocument();
    expect(mocked.setPassword).not.toHaveBeenCalled();
  });

  it.each(["/#password-setup=", "/", "/#unrelated=value"])("shows a complete invalid-link panel when the setup token is missing: %s", (url) => {
    window.history.replaceState({}, "", url);
    render(<PasswordSetupPage />);
    expect(screen.getByRole("heading", { name: "密码设置链接不可用" })).toHaveFocus();
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("联系组织管理员");
    expect(mocked.setPassword).not.toHaveBeenCalled();
  });

  it("shows required errors next to each field and focuses the first invalid field", () => {
    render(<PasswordSetupPage />);
    save();
    expect(screen.getByLabelText("新密码", { exact: true })).toHaveFocus();
    expect(screen.getByLabelText("新密码", { exact: true })).toHaveAccessibleDescription(/请输入新密码/);
    expect(screen.getByLabelText("确认密码", { exact: true })).toHaveAccessibleDescription("请再次输入新密码。");
    expect(mocked.setPassword).not.toHaveBeenCalled();
  });

  it.each(["a".repeat(11), "字".repeat(257)])("rejects password lengths outside the backend bounds (%#)", (password) => {
    render(<PasswordSetupPage />);
    fillPasswords(password);
    save();
    expect(screen.getByRole("alert")).toHaveTextContent("12–256");
    expect(screen.getByLabelText("新密码", { exact: true })).toHaveFocus();
    expect(mocked.setPassword).not.toHaveBeenCalled();
  });

  it("keeps the entered values and focuses confirmation on mismatch, then clears its error on editing", () => {
    render(<PasswordSetupPage />);
    fillPasswords(syntheticPassword, "different-example-password");
    save();
    expect(screen.getByLabelText("确认密码", { exact: true })).toHaveFocus();
    expect(screen.getByRole("alert")).toHaveTextContent("两次输入的密码不一致");
    expect(screen.getByLabelText("新密码", { exact: true })).toHaveValue(syntheticPassword);
    fireEvent.change(screen.getByLabelText("确认密码", { exact: true }), { target: { value: syntheticPassword } });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(mocked.setPassword).not.toHaveBeenCalled();
  });

  it("toggles each password independently without submitting or changing its value", () => {
    render(<PasswordSetupPage />);
    fillPasswords();
    fireEvent.click(screen.getByRole("button", { name: "显示新密码" }));
    expect(screen.getByLabelText("新密码", { exact: true })).toHaveAttribute("type", "text");
    expect(screen.getByRole("button", { name: "隐藏新密码" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByLabelText("确认密码", { exact: true })).toHaveAttribute("type", "password");
    fireEvent.click(screen.getByRole("button", { name: "隐藏新密码" }));
    expect(screen.getByLabelText("新密码", { exact: true })).toHaveValue(syntheticPassword);
    expect(screen.getByLabelText("新密码", { exact: true })).toHaveAttribute("type", "password");
    expect(mocked.setPassword).not.toHaveBeenCalled();
  });

  it.each(["界".repeat(12), "🔑".repeat(256), "  example-with-spaces  "])("uses Unicode character bounds and never trims a password (%#)", async (password) => {
    mocked.setPassword.mockResolvedValue(undefined);
    render(<PasswordSetupPage />);
    fillPasswords(password);
    save();
    await screen.findByRole("heading", { name: "密码设置成功" });
    expect(mocked.setPassword).toHaveBeenCalledExactlyOnceWith("isolated-layout-token", password);
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
  });

  it("uses a pending panel for the existing write request and removes fields on success", async () => {
    let finish!: () => void;
    mocked.setPassword.mockImplementation(() => new Promise<void>((resolve) => { finish = resolve; }));
    render(<StrictMode><PasswordSetupPage /></StrictMode>);
    fillPasswords();
    const form = screen.getByRole("form", { name: "设置登录密码" });
    fireEvent.submit(form);
    fireEvent.submit(form);
    expect(mocked.setPassword).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("heading", { name: "正在设置密码" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "正在保存…" })).toBeDisabled();
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
    await act(async () => finish());
    expect(screen.getByRole("heading", { name: "密码设置成功" })).toHaveFocus();
    expect(screen.getByRole("status")).toHaveTextContent("请返回登录");
    expect(screen.queryByLabelText("新密码", { exact: true })).not.toBeInTheDocument();
    expect(screen.queryByText("isolated-layout-token")).not.toBeInTheDocument();
    for (const link of screen.getAllByRole("link", { name: "返回登录" })) expect(link).toHaveAttribute("href", "/");
  });

  it("shows a link-specific result for a rejected setup link, not a login-password error", async () => {
    mocked.setPassword.mockRejectedValue(new ApiClientError(401, "unauthorized", "邮箱或密码错误，请重新输入。", "private-server-detail"));
    render(<PasswordSetupPage />);
    fillPasswords();
    save();
    expect(await screen.findByRole("heading", { name: "密码设置链接不可用" })).toHaveFocus();
    expect(screen.queryByRole("form")).not.toBeInTheDocument();
    expect(screen.queryByText(/邮箱或密码错误/)).not.toBeInTheDocument();
    expect(screen.queryByText("private-server-detail")).not.toBeInTheDocument();
  });

  it.each([0, 400, 429, 503])("returns to the populated form with a safe, focused error and supports explicit retry (%i)", async (status) => {
    mocked.setPassword.mockRejectedValueOnce(new ApiClientError(status, "request_failed", "请求未完成，请稍后重试。", "private-server-detail")).mockResolvedValueOnce(undefined);
    render(<PasswordSetupPage />);
    fillPasswords();
    save();
    expect(await screen.findByRole("alert")).toHaveFocus();
    expect(screen.getByLabelText("新密码", { exact: true })).toHaveValue(syntheticPassword);
    expect(screen.getByLabelText("确认密码", { exact: true })).toHaveValue(syntheticPassword);
    expect(screen.queryByText("private-server-detail")).not.toBeInTheDocument();
    save();
    expect(await screen.findByRole("heading", { name: "密码设置成功" })).toBeInTheDocument();
    expect(mocked.setPassword).toHaveBeenCalledTimes(2);
  });

  it("does not display unknown error objects or their raw messages", async () => {
    mocked.setPassword.mockRejectedValue(new Error("private-unexpected-detail"));
    render(<PasswordSetupPage />);
    fillPasswords();
    save();
    expect(await screen.findByRole("alert")).toHaveTextContent("暂时无法设置密码，请稍后重试。");
    expect(screen.queryByText("private-unexpected-detail")).not.toBeInTheDocument();
  });
});
