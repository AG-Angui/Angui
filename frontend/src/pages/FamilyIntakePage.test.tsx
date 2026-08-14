import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import { FamilyIntakePage } from "./FamilyIntakePage";

vi.mock("./FamilyIntakeForm", () => ({
  FamilyIntakeForm: ({ onCancel }: { onCancel: () => void }) => <div><button onClick={onCancel}>取消建案</button><p>确认提交前仍需家属二次确认</p></div>,
}));

describe("FamilyIntakePage", () => {
  it("renders the desktop step shell and mobile-safe intake guidance", () => {
    render(<MemoryRouter><FamilyIntakePage /></MemoryRouter>);
    expect(screen.getByRole("navigation", { name: "建案步骤" })).toBeInTheDocument();
    expect(screen.getByText("基本信息")).toBeInTheDocument();
    expect(screen.getByText("老人画像预览")).toBeInTheDocument();
    expect(screen.getByText("确认提交前仍需家属二次确认")).toBeInTheDocument();
    expect(screen.getByText(/不知道或不确定也可以继续/)).toBeInTheDocument();
  });
});
