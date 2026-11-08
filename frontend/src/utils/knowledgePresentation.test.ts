import { describe, expect, it } from "vitest";
import { presentKnowledgeItem } from "./knowledgePresentation";

describe("knowledge item presentation", () => {
  it("separates legacy identifiers, categories and source details from the body", () => {
    const item = {
      category: "Safety",
      content: "【经验ID】\nEXP-000134\n\n【经验标题】\nNew volunteers\n\n【一级分类】\nSafety\n\n【二级分类】\nTraining\n\n【判断结论】\nProvide support\n\n【推荐动作】\nAssign a mentor\n\n【原始回答】\nOriginal interview answer",
    };
    const presentation = presentKnowledgeItem(item);
    expect(presentation.externalId).toBe("EXP-000134");
    expect(presentation.category).toBe("Safety");
    expect(presentation.subcategory).toBe("Training");
    expect(presentation.body).toBe("【判断结论】\nProvide support\n\n【推荐动作】\nAssign a mentor");
    expect(presentation.body).not.toContain("EXP-000134");
    expect(presentation.sourceDetails).toEqual([["原始回答", "Original interview answer"]]);
    expect(item.content).toContain("EXP-000134");
  });

  it("uses structured fields while preserving an ordinary article exactly", () => {
    const body = "Introduction\n\n【判断结论】\nKeep this article intact";
    const presentation = presentKnowledgeItem({
      content: body,
      category: "Safety",
      external_id: "EXP-000001",
      metadata: {
        primary_category: "Safety",
        secondary_category: "Training",
        source_fields: { "经验ID": "old-id", "原始回答": "Source answer" },
      },
    });
    expect(presentation.body).toBe(body);
    expect(presentation.externalId).toBe("EXP-000001");
    expect(presentation.subcategory).toBe("Training");
    expect(presentation.sourceDetails).toEqual([["原始回答", "Source answer"]]);
  });
});
