import type { KnowledgeItem } from "../api/learning";

export const experienceFields = [
  "经验ID",
  "经验标题",
  "一级分类",
  "二级分类",
  "适用场景",
  "老人特征",
  "前置信号",
  "判断结论",
  "判断依据",
  "推荐动作",
  "优先级",
  "不适用条件",
  "常见误区",
  "风险提醒",
  "案例依据",
  "经验来源",
  "验证程度",
  "原始访谈",
  "原始回答",
  "署名方式",
  "来源问卷",
] as const;
export type ExperienceField = (typeof experienceFields)[number];
export type FieldItem = Pick<
  KnowledgeItem,
  | "title"
  | "content"
  | "category"
  | "external_id"
  | "metadata"
  | "source_name"
  | "summary"
  | "keywords"
>;
export function object(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}
export function fieldText(value: unknown): string {
  return value == null
    ? ""
    : typeof value === "string"
      ? value
      : JSON.stringify(value);
}
export function knowledgeFields(
  item: FieldItem,
  compactTitle = true,
): Record<string, string> {
  const sections: Record<string, string> = {};
  const matches = [...item.content.matchAll(/^【([^】\r\n]+)】\r?\n/gm)];
  if (matches[0]?.index === 0) {
    matches.forEach((match, index) => {
      sections[match[1]] = item.content
        .slice(match.index! + match[0].length, matches[index + 1]?.index)
        .trim();
    });
  }
  const metadata = object(item.metadata);
  const source = object(metadata.source_fields);
  const fields = Object.fromEntries(
    Object.entries({ ...sections, ...source }).map(([key, value]) => [
      key,
      fieldText(value),
    ]),
  );
  fields["经验ID"] = item.external_id || fields["经验ID"] || "";
  fields["经验标题"] = item.title || fields["经验标题"] || "";
  fields["一级分类"] =
    fieldText(metadata.primary_category) || fields["一级分类"] || item.category;
  fields["二级分类"] =
    fieldText(metadata.secondary_category) || fields["二级分类"] || "";
  fields["来源问卷"] = item.source_name || fields["来源问卷"] || "";
  fields["风险提醒"] ||= fields["风险提示"] || "";
  fields["适用场景"] ||= item.summary;
  if (matches.length === 0 && !fields["判断结论"])
    fields["判断结论"] = item.content;
  // Generated experience titles repeat their answer after a colon. Display the
  // short heading; keep the complete source title available in the cell tooltip.
  if (!compactTitle) return fields;
  const heading = fields["经验标题"].split(/[：:]/, 1)[0].trim();
  if (heading && ![fields["一级分类"], fields["二级分类"]].includes(heading)) {
    fields["经验标题"] = heading;
  } else {
    fields["经验标题"] = fields["经验标题"].replace(/^[^：:]+[：:]\s*/, "");
  }
  return fields;
}

export const csvHeaders = [
  "knowledge_base_id",
  "title",
  "content",
  "summary",
  "category",
  "keywords",
  "source_name",
  "source_url",
  "visibility",
  "external_id",
  "metadata",
];
export function serializeKnowledgeCsv(rows: string[][]): string {
  return [csvHeaders, ...rows]
    .map((row) =>
      row.map((value) => '"' + value.replaceAll('"', '""') + '"').join(","),
    )
    .join("\r\n");
}
export function csvRowItem(cells: string[]): FieldItem {
  let metadata: Record<string, unknown> = {};
  try {
    metadata = object(JSON.parse(cells[10] || "{}"));
  } catch {
    /* Revalidation reports invalid JSON. */
  }
  return {
    title: cells[1] || "",
    content: cells[2] || "",
    summary: cells[3] || "",
    category: cells[4] || "",
    keywords: (cells[5] || "").split(","),
    source_name: cells[6] || "",
    external_id: cells[9] || "",
    metadata,
  };
}
export function editCsvField(
  cells: string[],
  field: ExperienceField,
  value: string,
): string[] {
  const next = Array.from({ length: 11 }, (_, index) => cells[index] || "");
  const item = csvRowItem(cells);
  const metadata = { ...object(item.metadata) };
  // Keep every source field when converting a legacy labeled record.
  const source = {
    ...knowledgeFields(item, false),
    ...object(metadata.source_fields),
    [field]: value,
  };
  metadata.source_fields = source;
  const index = (
    {
      经验ID: 9,
      经验标题: 1,
      一级分类: 4,
      来源问卷: 6,
      适用场景: 3,
    } as Partial<Record<ExperienceField, number>>
  )[field];
  if (index !== undefined) next[index] = value;
  if (field === "一级分类") metadata.primary_category = value;
  if (field === "二级分类") metadata.secondary_category = value;
  next[2] =
    ["判断结论", "推荐动作", "风险提醒"]
      .filter((key) => fieldText(source[key as keyof typeof source]))
      .map(
        (key) =>
          "【" + key + "】\n" + fieldText(source[key as keyof typeof source]),
      )
      .join("\n\n") || item.content;
  next[10] = JSON.stringify(metadata);
  return next;
}
