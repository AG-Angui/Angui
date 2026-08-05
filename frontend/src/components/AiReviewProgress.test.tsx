import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AiReviewProgress } from "./AiReviewProgress";

describe("AiReviewProgress", () => {
  it("labels a generated candidate as an unconfirmed draft", () => {
    render(<AiReviewProgress stage="generating" title="AI 初步审核进行中" />);

    expect(screen.getByRole("status")).toHaveAccessibleName(
      "AI 初步审核进行中：正在生成审核候选",
    );
    expect(
      screen.getByText("生成的内容仍是草稿，尚未成为已确认事实。"),
    ).toBeInTheDocument();
  });

  it("explains the deterministic fallback without presenting it as model output", () => {
    render(<AiReviewProgress stage="fallback" />);

    expect(screen.getByText("正在切换规则结果")).toBeInTheDocument();
    expect(
      screen.getByText(/AI 结果不可用或未通过校验/),
    ).toBeInTheDocument();
  });
});
