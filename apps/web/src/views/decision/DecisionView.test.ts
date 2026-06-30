import { describe, it, expect } from "vitest";

// ----- splitAuditNote -----

function splitAuditNote(note: string): string[] {
  const normalized = note.replace(/\s+/g, " ").trim();
  const segments = normalized
    .split(/(?<=。)\s*|(?<=；)\s*/)
    .map((segment) => segment.trim())
    .filter((segment) => segment.length > 0);
  return segments.length > 0 ? segments : [normalized];
}

describe("splitAuditNote", () => {
  it("splits by Chinese period", () => {
    const result = splitAuditNote("数据来源 API。更新频率日频。");
    expect(result).toHaveLength(2);
    expect(result[0]).toBe("数据来源 API。");
    expect(result[1]).toBe("更新频率日频。");
  });

  it("splits by Chinese semicolon", () => {
    const result = splitAuditNote("评分输入，不是危机概率；规则层 0-100 分。");
    expect(result).toHaveLength(2);
    expect(result[0]).toBe("评分输入，不是危机概率；");
    expect(result[1]).toBe("规则层 0-100 分。");
  });

  it("handles single segment without punctuation", () => {
    const result = splitAuditNote("单一口径说明");
    expect(result).toEqual(["单一口径说明"]);
  });

  it("normalizes whitespace", () => {
    const result = splitAuditNote("来源  API。  说明  文本。");
    expect(result).toHaveLength(2);
    expect(result[0]).toBe("来源 API。");
    expect(result[1]).toBe("说明 文本。");
  });

  it("returns single segment when splitting produces empty", () => {
    expect(splitAuditNote("  ")).toEqual([""]);
  });

  it("handles mixed punctuation", () => {
    const result = splitAuditNote("第一段。第二段；第三段。");
    expect(result).toHaveLength(3);
  });
});

// ----- splitAuditDetail -----

function splitAuditDetail(detail: string): string[] {
  const segments = detail
    .split(/\s+\/\s+|[；，]| · /)
    .map((segment) => segment.trim())
    .filter(Boolean);
  return segments.length > 0 ? segments : [detail];
}

describe("splitAuditDetail", () => {
  it("splits by slash with spaces", () => {
    expect(splitAuditDetail("A / B / C")).toEqual(["A", "B", "C"]);
  });

  it("splits by Chinese semicolon", () => {
    expect(splitAuditDetail("第一项；第二项；第三项")).toEqual(["第一项", "第二项", "第三项"]);
  });

  it("splits by Chinese comma", () => {
    expect(splitAuditDetail("甲，乙，丙")).toEqual(["甲", "乙", "丙"]);
  });

  it("splits by spaced dot", () => {
    expect(splitAuditDetail("X · Y · Z")).toEqual(["X", "Y", "Z"]);
  });

  it("returns original when no separator found", () => {
    expect(splitAuditDetail("single segment")).toEqual(["single segment"]);
  });

  it("handles segments with varied spacing", () => {
    const result = splitAuditDetail("A / B");
    expect(result).toContain("A");
    expect(result).toContain("B");
  });

  it("splits chinese comma with mixed content", () => {
    expect(splitAuditDetail("评分输入，不是危机概率")).toEqual(["评分输入", "不是危机概率"]);
  });
});

// ----- auditSegmentClassName -----

const NUMBER_TOKEN_PATTERN =
  /(\d{4}-\d{2}-\d{2}(?:\s+\d{2}:\d{2}(?::\d{2})?)?|[+-]?\d+(?:\.\d+)?\s?-\s?[+-]?\d+(?:\.\d+)?|[+-]?\d{1,3}(?:,\d{3})*(?:\.\d+)?\s?(?:%|bp|bps|d|分|条|天)?)/g;

const NUMBER_TOKEN_EXACT_PATTERN =
  /^(\d{4}-\d{2}-\d{2}(?:\s+\d{2}:\d{2}(?::\d{2})?)?|[+-]?\d+(?:\.\d+)?\s?-\s?[+-]?\d+(?:\.\d+)?|[+-]?\d{1,3}(?:,\d{3})*(?:\.\d+)?\s?(?:%|bp|bps|d|分|条|天)?)$/;

function auditSegmentClassName(segment: string): string {
  const hasHighlightedValue = segment
    .split(NUMBER_TOKEN_PATTERN)
    .some((part) => NUMBER_TOKEN_EXACT_PATTERN.test(part));
  return hasHighlightedValue
    ? "number-audit-segment"
    : "number-audit-segment number-audit-segment-copy";
}

describe("auditSegmentClassName", () => {
  it("returns copy-only class for plain text", () => {
    const result = auditSegmentClassName("纯文本段");
    expect(result).toContain("number-audit-segment-copy");
  });

  it("returns highlight class for segment with number", () => {
    const result = auditSegmentClassName("分数 36.0");
    expect(result).toBe("number-audit-segment");
  });

  it("returns highlight class for segment with percentage", () => {
    const result = auditSegmentClassName("风险 45%");
    expect(result).toBe("number-audit-segment");
  });

  it("returns highlight class for segment with date", () => {
    const result = auditSegmentClassName("截至 2026-06-30");
    expect(result).toBe("number-audit-segment");
  });

  it("returns highlight class for segment with range", () => {
    const result = auditSegmentClassName("区间 10-20d");
    expect(result).toBe("number-audit-segment");
  });
});

// ----- changeValueToneClassName -----

function changeValueToneClassName(value: string) {
  const trimmed = value.trim();
  if (trimmed.startsWith("+")) {
    return "numeric-value-up";
  }
  if (trimmed.startsWith("-")) {
    return "numeric-value-down";
  }
  return "numeric-value-flat";
}

describe("changeValueToneClassName", () => {
  it("returns up for positive values", () => {
    expect(changeValueToneClassName("+5")).toBe("numeric-value-up");
    expect(changeValueToneClassName("+0.1%")).toBe("numeric-value-up");
  });

  it("returns down for negative values", () => {
    expect(changeValueToneClassName("-5")).toBe("numeric-value-down");
    expect(changeValueToneClassName("-0.1%")).toBe("numeric-value-down");
  });

  it("returns flat for values without sign", () => {
    expect(changeValueToneClassName("0")).toBe("numeric-value-flat");
    expect(changeValueToneClassName("36.0")).toBe("numeric-value-flat");
  });
});

// ----- numberAuditToneClass -----

function numberAuditToneClass(rowId: string): string {
  switch (rowId) {
    case "mvp-state":
      return "number-audit-tone-primary";
    case "risk-score-snapshot":
    case "event-confirmation":
    case "jpy-carry":
      return "number-audit-tone-risk";
    case "decision-reliability":
    case "probability-snapshot":
      return "number-audit-tone-reference";
    case "usdjpy":
    case "freshness":
      return "number-audit-tone-data";
    case "position-guidance":
      return "number-audit-tone-action";
    default:
      return "number-audit-tone-neutral";
  }
}

describe("numberAuditToneClass", () => {
  it("returns primary for mvp-state", () => {
    expect(numberAuditToneClass("mvp-state")).toBe("number-audit-tone-primary");
  });

  it("returns risk for score/event/carry", () => {
    expect(numberAuditToneClass("risk-score-snapshot")).toBe("number-audit-tone-risk");
    expect(numberAuditToneClass("event-confirmation")).toBe("number-audit-tone-risk");
    expect(numberAuditToneClass("jpy-carry")).toBe("number-audit-tone-risk");
  });

  it("returns reference for reliability/probability", () => {
    expect(numberAuditToneClass("decision-reliability")).toBe("number-audit-tone-reference");
    expect(numberAuditToneClass("probability-snapshot")).toBe("number-audit-tone-reference");
  });

  it("returns data for usdjpy/freshness", () => {
    expect(numberAuditToneClass("usdjpy")).toBe("number-audit-tone-data");
    expect(numberAuditToneClass("freshness")).toBe("number-audit-tone-data");
  });

  it("returns action for position-guidance", () => {
    expect(numberAuditToneClass("position-guidance")).toBe("number-audit-tone-action");
  });

  it("returns neutral for unknown ids", () => {
    expect(numberAuditToneClass("unknown-row")).toBe("number-audit-tone-neutral");
    expect(numberAuditToneClass("")).toBe("number-audit-tone-neutral");
  });
});
