import { describe, it, expect } from "vitest";

// ----- formatErrorText -----

function formatErrorText(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string" && error.trim().length > 0) {
    return error;
  }
  return "未知错误";
}

describe("formatErrorText", () => {
  it("returns Error message for Error instances", () => {
    expect(formatErrorText(new Error("API failed"))).toBe("API failed");
  });

  it("returns string directly for non-empty strings", () => {
    expect(formatErrorText("custom error")).toBe("custom error");
  });

  it("returns fallback for empty strings", () => {
    expect(formatErrorText("  ")).toBe("未知错误");
  });

  it("returns fallback for null", () => {
    expect(formatErrorText(null)).toBe("未知错误");
  });

  it("returns fallback for undefined", () => {
    expect(formatErrorText(undefined)).toBe("未知错误");
  });

  it("returns fallback for objects", () => {
    expect(formatErrorText({ code: 500 })).toBe("未知错误");
  });
});

// ----- isView -----

type View = "overview" | "indicators" | "alerts" | "sources" | "backtests" | "decision";
const navItems = [
  { id: "overview" as const, label: "总览" },
  { id: "decision" as const, label: "决策面板" },
  { id: "indicators" as const, label: "指标" },
  { id: "alerts" as const, label: "警报" },
  { id: "sources" as const, label: "数据源" },
  { id: "backtests" as const, label: "回测" }
];

function isView(value: string | null): value is View {
  return value !== null && navItems.some((item) => item.id === value);
}

describe("isView", () => {
  it("returns true for valid view names", () => {
    expect(isView("decision")).toBe(true);
    expect(isView("overview")).toBe(true);
    expect(isView("backtests")).toBe(true);
  });

  it("returns false for invalid view names", () => {
    expect(isView("invalid")).toBe(false);
    expect(isView("")).toBe(false);
  });

  it("returns false for null", () => {
    expect(isView(null)).toBe(false);
  });
});

// ----- firstQueryError -----

const VIEW_DATA_KEYS: Record<string, string[]> = {
  decision: ["assessment", "riskThresholds"],
  overview: ["overview", "indicators"]
};

function firstQueryError(
  data: Record<string, unknown>,
  queryErrors: Partial<Record<string, unknown>>,
  view: string
): unknown {
  const requiredKeys = VIEW_DATA_KEYS[view];
  return requiredKeys
    .map((key) => queryErrors[key])
    .find((value) => value !== null && value !== undefined);
}

describe("firstQueryError", () => {
  it("returns first non-null error for required keys", () => {
    const errors = { assessment: null, riskThresholds: new Error("thresholds failed") };
    expect(firstQueryError({}, errors, "decision")).toBeInstanceOf(Error);
  });

  it("returns undefined when no errors", () => {
    const errors = { assessment: null, riskThresholds: null };
    expect(firstQueryError({}, errors, "decision")).toBeUndefined();
  });

  it("returns undefined for missing key errors", () => {
    const errors = {};
    expect(firstQueryError({}, errors, "decision")).toBeUndefined();
  });
});

// ----- productionSourceIssueLabels -----

interface MockSource {
  production_allowed: boolean;
  display_name: string;
  health: { status: string };
}

function productionSourceIssueLabels(sources: MockSource[] | null | undefined): string[] {
  return (
    sources
      ?.filter(
        (source) =>
          source.production_allowed &&
          ["delayed", "partial_failure", "failed"].includes(source.health.status)
      )
      .map((source) => source.display_name) ?? []
  );
}

describe("productionSourceIssueLabels", () => {
  it("returns labels for degraded production sources", () => {
    const sources: MockSource[] = [
      { production_allowed: true, display_name: "FRED", health: { status: "delayed" } },
      { production_allowed: true, display_name: "BOJ", health: { status: "healthy" } },
      { production_allowed: false, display_name: "Alpha Vantage", health: { status: "failed" } }
    ];
    expect(productionSourceIssueLabels(sources)).toEqual(["FRED"]);
  });

  it("returns empty array for null/undefined", () => {
    expect(productionSourceIssueLabels(null)).toEqual([]);
    expect(productionSourceIssueLabels(undefined)).toEqual([]);
  });

  it("returns empty array for healthy sources", () => {
    const sources: MockSource[] = [
      { production_allowed: true, display_name: "FRED", health: { status: "healthy" } }
    ];
    expect(productionSourceIssueLabels(sources)).toEqual([]);
  });

  it("handles partial_failure and failed statuses", () => {
    const sources: MockSource[] = [
      { production_allowed: true, display_name: "Source A", health: { status: "partial_failure" } },
      { production_allowed: true, display_name: "Source B", health: { status: "failed" } },
      { production_allowed: true, display_name: "Source C", health: { status: "delayed" } }
    ];
    expect(productionSourceIssueLabels(sources)).toEqual(["Source A", "Source B", "Source C"]);
  });
});
