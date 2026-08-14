import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AuthContextValue } from "../auth/auth-context";
import { LoginPage } from "./LoginPage";

const mocked = vi.hoisted(() => ({
  auth: null as AuthContextValue | null,
}));

vi.mock("../auth/useAuth", () => ({ useAuth: () => mocked.auth }));

function setAuth(
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
}

describe("LoginPage", () => {
  it("shows field errors and returns focus to the first missing field", () => {
    setAuth();
    render(<LoginPage />);

    fireEvent.click(screen.getByRole("button", { name: "登录" }));

    expect(screen.getByText("请输入邮箱地址。")).toHaveAttribute(
      "role",
      "alert",
    );
    expect(screen.getByLabelText("邮箱")).toHaveFocus();
  });

  it("lets people reveal or conceal their password while typing", () => {
    setAuth();
    render(<LoginPage />);

    const passwordInput = screen.getByLabelText("密码");
    expect(passwordInput).toHaveAttribute("type", "password");

    fireEvent.click(screen.getByRole("button", { name: "显示密码" }));
    expect(passwordInput).toHaveAttribute("type", "text");

    fireEvent.click(screen.getByRole("button", { name: "隐藏密码" }));
    expect(passwordInput).toHaveAttribute("type", "password");
  });

  it("uses a trimmed email and removes a server error when credentials change", async () => {
    const login = vi
      .fn()
      .mockRejectedValue(new Error("邮箱或密码错误，请重新输入。"));
    setAuth(login);
    render(<LoginPage />);

    fireEvent.change(screen.getByLabelText("邮箱"), {
      target: { value: " family@demo.invalid " },
    });
    fireEvent.change(screen.getByLabelText("密码"), {
      target: { value: "local-test-input" },
    });
    fireEvent.click(screen.getByRole("button", { name: "登录" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "邮箱或密码错误，请重新输入。",
    );
    expect(login).toHaveBeenCalledWith(
      "family@demo.invalid",
      "local-test-input",
    );

    fireEvent.change(screen.getByLabelText("密码"), {
      target: { value: "new-input" },
    });
    await waitFor(() =>
      expect(
        screen.queryByText("邮箱或密码错误，请重新输入。"),
      ).not.toBeInTheDocument(),
    );
  });

  it("shows the unified role guide and access request entry", () => {
    setAuth();
    render(<LoginPage />);
    expect(screen.getByText("面向失智老人走失搜救的协同入口")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /家属/ })).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(screen.getByRole("button", { name: /志愿者/ }));
    expect(screen.getByRole("button", { name: /志愿者/ })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("link", { name: "申请访问" })).toHaveAttribute("href", "/access-request");
  });
});
