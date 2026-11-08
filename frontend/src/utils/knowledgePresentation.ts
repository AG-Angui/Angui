import type { KnowledgeItem } from "../api/learning";

const bodyFields = ["判断结论", "推荐动作", "适用场景", "风险提示"];
const separateFields = new Set([
  "经验ID",
  "经验标题",
  "一级分类",
  "二级分类",
  "来源问卷",
  ...bodyFields,
]);

function sourceFields(value: unknown): Record<string, unknown> {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function text(value: unknown): string {
  return typeof value === "string" ? value : "";
}

// Older CSV exports embedded the source record in labeled content sections.
// Only recognize that complete record format; ordinary articles stay intact.
export function presentKnowledgeItem(
  item: Pick<KnowledgeItem, "content" | "external_id" | "category" | "metadata">,
) {
  const matches = [...item.content.matchAll(/^【([^】\r\n]+)】\r?\n/gm)];
  const legacy: Record<string, string> = {};
  if (
    matches[0]?.index === 0 &&
    matches[0][1] === "经验ID" &&
    matches.some((match) => match[1] === "经验标题")
  ) {
    matches.forEach((match, index) => {
      legacy[match[1]] = item.content.slice(
        match.index! + match[0].length,
        matches[index + 1]?.index,
      ).trim();
    });
  }
  const fields = { ...legacy, ...sourceFields(item.metadata?.source_fields) };
  const body = Object.keys(legacy).length > 0
    ? bodyFields
        .filter((key) => text(fields[key]))
        .map((key) => `【${key}】\n${text(fields[key])}`)
        .join("\n\n") || item.content
    : item.content;
  return {
    externalId: item.external_id || text(fields["经验ID"]),
    category:
      text(item.metadata?.primary_category) || text(fields["一级分类"]) || item.category,
    subcategory:
      text(item.metadata?.secondary_category) || text(fields["二级分类"]),
    body,
    sourceDetails: Object.entries({ ...item.metadata, ...fields }).filter(
      ([key]) =>
        !separateFields.has(key) &&
        !["primary_category", "secondary_category", "source_fields"].includes(key),
    ),
  };
}
