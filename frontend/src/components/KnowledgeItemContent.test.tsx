import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import { KnowledgeItemContent } from "./KnowledgeItemContent";

it("keeps original answers collapsed while allowing the full source to be opened", () => {
  render(<KnowledgeItemContent presentation={{
    externalId: "EXP-000001", category: "Safety", subcategory: "Training",
    body: "Provide support", sourceDetails: [["原始回答", "Original answer"]],
  }} />);
  const source = screen.getByText("Original answer");
  expect(source).not.toBeVisible();
  fireEvent.click(screen.getByText("查看完整正文与原始资料"));
  expect(source).toBeVisible();
});
