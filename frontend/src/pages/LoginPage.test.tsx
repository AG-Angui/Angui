import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import { ApiClientError } from "../api/client";
import type { AuthContextValue } from "../auth/auth-context";
import { LoginPage } from "./LoginPage";

const mocked = vi.hoisted(() => ({
  auth: null as AuthContextValue | null,
}));

vi.mock("../auth/useAuth", () => ({ useAuth: () => mocked.auth }));

function renderLogin(
  login: AuthContextValue["login"] = vi.fn().mockResolvedValue(undefined),
) {
  mocked.auth = {
    user: null,
    token: null,
    isLoading: false,
    isLoggingOut: false,
    sessionNotice: null,
    login,
    logout: vi.fn(),
    refreshUser: vi.fn(),
  };
  render(<MemoryRouter><LoginPage /></MemoryRouter>);
  return login;
}

function openFamily() {
  const trigger = screen.getByRole("button", { name: /家属 \/ 知情人/ });
  trigger.focus();
  fireEvent.click(trigger);
  return trigger;
}

function fillCredentials() {
  fireEvent.change(screen.getByLabelText("邮箱"), {
    target: { value: " family@demo.invalid " },
  });
  fireEvent.change(screen.getByLabelText("密码"), {
    target: { value: "local-test-input" },
  });
}

describe("LoginPage", () => {
  it("initially shows three desktop entries, not a permanently exposed form", () => {
    renderLogin();
    expect(screen.getByRole("heading", { level: 1, name: "安归 · 统一身份入口" })).toBeInTheDocument();
    expect(screen.queryByText("让求助有人跟进，让行动有序协同")).not.toBeInTheDocument();
    expect(screen.queryByText(/连接家属、志愿者与指挥人员/)).not.toBeInTheDocument();
    expect(screen.queryByText(/共同帮助失联老人尽早平安回家/)).not.toBeInTheDocument();
    const entries = screen.getAllByRole("button");
    expect(entries).toHaveLength(3);
    expect(entries[0]).toHaveTextContent("志愿者");
    expect(entries[1]).toHaveTextContent("家属 / 知情人");
    expect(entries[2]).toHaveTextContent("指挥 / 管理人员");
    expect(screen.queryByLabelText("邮箱")).not.toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("puts the family entry first in the actual mobile DOM order", () => {
    vi.stubGlobal("matchMedia", vi.fn(() => ({
      matches: true,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })));
    renderLogin();
    const entries = screen.getAllByRole("button");
    expect(entries[0]).toHaveTextContent("家属 / 知情人");
    expect(entries[1]).toHaveTextContent("志愿者");
    expect(entries[2]).toHaveTextContent("指挥 / 管理人员");
  });

  it("shows field errors and returns focus to the first missing field", () => {
    renderLogin();
    openFamily();
    fireEvent.click(screen.getByRole("button", { name: "登录" }));
    expect(screen.getByText("请输入邮箱地址。")).toHaveAttribute("role", "alert");
    expect(screen.getByLabelText("邮箱")).toHaveFocus();
  });

  it("lets people reveal or conceal their password while typing", () => {
    renderLogin();
    openFamily();
    const passwordInput = screen.getByLabelText("密码");
    expect(passwordInput).toHaveAttribute("type", "password");
    fireEvent.click(screen.getByRole("button", { name: "显示密码" }));
    expect(passwordInput).toHaveAttribute("type", "text");
    fireEvent.click(screen.getByRole("button", { name: "隐藏密码" }));
    expect(passwordInput).toHaveAttribute("type", "password");
  });

  it("uses a trimmed email and removes a server error when credentials change", async () => {
    const login = vi.fn().mockRejectedValue(new Error("邮箱或密码错误，请重新输入。"));
    renderLogin(login);
    openFamily();
    fillCredentials();
    fireEvent.click(screen.getByRole("button", { name: "登录" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("邮箱或密码错误，请重新输入。");
    expect(login).toHaveBeenCalledWith("family@demo.invalid", "local-test-input");
    fireEvent.change(screen.getByLabelText("密码"), { target: { value: "new-input" } });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("opens the selected role dialog with the real access-request link", () => {
    renderLogin();
    fireEvent.click(screen.getByRole("button", { name: /志愿者/ }));
    expect(screen.getByRole("dialog", { name: "志愿者登录" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "申请访问" })).toHaveAttribute("href", "/access-request?role=volunteer");
    // Background entries must not remain exposed through the accessibility tree.
    expect(screen.queryByRole("button", { name: /家属 \/ 知情人/ })).not.toBeInTheDocument();
  });

  it("closes on Escape, restores the trigger focus, and clears only the password", async () => {
    renderLogin();
    const trigger = openFamily();
    fillCredentials();
    fireEvent.click(screen.getByRole("button", { name: "显示密码" }));
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape", code: "Escape" });
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
    fireEvent.click(screen.getByRole("button", { name: /志愿者/ }));
    expect(screen.getByLabelText("邮箱")).toHaveValue("family@demo.invalid");
    expect(screen.getByLabelText("密码")).toHaveValue("");
    expect(screen.getByLabelText("密码")).toHaveAttribute("type", "password");
  });

  it("returns to identity selection using the explicit back action", async () => {
    renderLogin();
    openFamily();
    fireEvent.click(screen.getByRole("button", { name: "返回身份选择" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(screen.getAllByRole("button")).toHaveLength(3);
  });

  it("blocks duplicate submission and dismissal while real authentication is pending", async () => {
    let finish!: () => void;
    const login = vi.fn(() => new Promise<void>((resolve) => { finish = resolve; }));
    renderLogin(login);
    openFamily();
    fillCredentials();
    const form = screen.getByRole("form", { name: "身份登录" });
    fireEvent.submit(form);
    fireEvent.submit(form);
    expect(login).toHaveBeenCalledTimes(1);
    expect(form).toHaveAttribute("aria-busy", "true");
    expect(screen.getByRole("button", { name: "返回身份选择" })).toBeDisabled();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape", code: "Escape" });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    await act(async () => { finish(); });
    expect(form).toHaveAttribute("aria-busy", "false");
  });

  it("offers retry for connection failures without discarding credentials", async () => {
    const login = vi.fn().mockRejectedValue(new ApiClientError(0, "network_error", "网络连接失败，请检查服务连接后重试。"));
    renderLogin(login);
    openFamily();
    fillCredentials();
    fireEvent.click(screen.getByRole("button", { name: "登录" }));
    const retry = await screen.findByRole("button", { name: "重试登录" });
    expect(screen.getByLabelText("密码")).toHaveValue("local-test-input");
    fireEvent.click(retry);
    await waitFor(() => expect(login).toHaveBeenCalledTimes(2));
  });

  it.each([401, 403])("does not invent an account-review state for HTTP %i", async (status) => {
    const login = vi.fn().mockRejectedValue(new ApiClientError(status, "denied", "Server detail must not be displayed"));
    renderLogin(login);
    fireEvent.click(screen.getByRole("button", { name: /指挥 \/ 管理人员/ }));
    fillCredentials();
    fireEvent.click(screen.getByRole("button", { name: "登录" }));
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(status === 401 ? "邮箱或密码错误" : "当前请求未获授权");
    expect(alert).not.toHaveTextContent("Server detail");
    expect(login).toHaveBeenCalledWith("family@demo.invalid", "local-test-input");
    expect(screen.queryByRole("button", { name: "重试登录" })).not.toBeInTheDocument();
  });
});
