import { describe, expect, it } from "vitest";
import {
  csvRowItem,
  editCsvField,
  knowledgeFields,
  serializeKnowledgeCsv,
} from "./knowledgeFields";

const row = [
  "learning-materials",
  "新人参与边界与支持：不应强制安排新人出任务",
  "【判断结论】\n不应强制安排新人出任务\n\n【推荐动作】\n主力带新人",
  "",
  "志愿者安全经验",
  "新人",
  "来源问卷",
  "",
  "learner",
  "EXP-1",
  JSON.stringify({
    source_fields: { 优先级: "待人工评估", 原始回答: "原始答复" },
  }),
];

describe("experience field mapping", () => {
  it("separates guidance, source answers and title without changing the record", () => {
    const fields = knowledgeFields(csvRowItem(row));
    expect(fields["经验标题"]).toBe("新人参与边界与支持");
    expect(fields["判断结论"]).toBe("不应强制安排新人出任务");
    expect(fields["推荐动作"]).toBe("主力带新人");
    expect(fields["优先级"]).toBe("待人工评估");
    expect(fields["原始回答"]).toBe("原始答复");
    expect(row[1]).toContain("不应强制安排");
  });
  it("retains the full source title and unknown metadata during unrelated edits", () => {
    const edited = editCsvField(row, "优先级", "优先支持");
    const source = JSON.parse(edited[10]).source_fields;
    expect(source["经验标题"]).toBe(row[1]);
    expect(source["原始回答"]).toBe("原始答复");
    expect(source["优先级"]).toBe("优先支持");
    expect(edited[1]).toBe(row[1]);
    expect(edited[2]).toContain("主力带新人");
  });
  it("updates the stored body when guidance changes and escapes CSV correctly", () => {
    const edited = editCsvField(row, "推荐动作", '先培训，再带教\n保留"原话"');
    expect(edited[2]).toContain('先培训，再带教\n保留"原话"');
    expect(serializeKnowledgeCsv([edited])).toContain('保留""原话""');
    expect(row[2]).toContain("主力带新人");
  });
});
