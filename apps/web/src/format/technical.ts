function humanizeTechnicalFamily(family: string) {
  const mappings: Array<[RegExp, (...groups: string[]) => string]> = [
    [/^scoring_v(\d+)$/, (version) => `评分规则 v${version}`],
    [/^interaction_tail_v(\d+)$/, (version) => `概率模型 v${version}`],
    [/^interaction_tail_extmix(\d+)$/, (variant) => `候选版本 ${variant}`],
    [/^platt$/, () => "Platt 校准"],
    [/^formal_v(\d+)_main$/, (version) => `正式特征主线 v${version}`],
    [/^formal_label_v(\d+)$/, (version) => `正式标签口径 v${version}`],
    [/^posture_v(\d+)$/, (version) => `执行节奏规则 v${version}`],
    [/^action_playbook_v(\d+)$/, (version) => `动作框架 v${version}`],
    [/^runtime_history_v(\d+)$/, (version) => `历史审计策略 v${version}`],
    [/^protected_stress_windows$/, () => "受保护窗口目录"]
  ];

  for (const [pattern, formatter] of mappings) {
    const match = family.match(pattern);
    if (match) {
      return formatter(...match.slice(1));
    }
  }

  return family;
}

export function compactTechnicalId(
  value: string | null | undefined,
  familySegmentCount = 3
) {
  if (!value) {
    return {
      value: "none",
      hint: undefined
    };
  }

  const [head] = value.split("|");
  const parts = head.split("_").filter(Boolean);
  if (parts.length === 2 && /^\d{8}(T\d+)?$/i.test(parts[1])) {
    return {
      value: `${humanizeTechnicalFamily(parts[0])} · ${parts[1]}`,
      hint: value
    };
  }

  if (parts.length < 3) {
    return {
      value: head,
      hint: head === value ? undefined : value
    };
  }

  const timestamp = parts.at(-1);
  const family = humanizeTechnicalFamily(
    parts
      .slice(Math.max(0, parts.length - (familySegmentCount + 1)), parts.length - 1)
      .join("_")
  );
  return {
    value: `${family} · ${timestamp}`,
    hint: value
  };
}

export function releaseIdLabel(value: string | null | undefined) {
  if (!value) {
    return {
      value: "未绑定版本",
      hint: undefined
    };
  }

  const [head] = value.split("|");
  const extmixMatch = head.match(/(?:^|_)(?:main_)?extmix(\d*)_(\d{8})(?:T(\d{2})(\d{2})(\d{2}))?$/);
  const mainMatch = head.match(/(?:^|_)main_(\d{8})(?:T(\d{2})(\d{2})(\d{2}))?$/);
  const formatTimestamp = (date: string, hour?: string, minute?: string) => {
    const formattedDate = `${date.slice(0, 4)}-${date.slice(4, 6)}-${date.slice(6, 8)}`;
    return `${formattedDate}${hour && minute ? ` ${hour}:${minute}` : ""}`;
  };

  if (extmixMatch) {
    const [, version, date, hour, minute] = extmixMatch;
    return {
      value: `${version ? `候选版本 ${version}` : "候选版本"} · ${formatTimestamp(date, hour, minute)}`,
      hint: value
    };
  }

  if (mainMatch) {
    const [, date, hour, minute] = mainMatch;
    return {
      value: `主线版本 · ${formatTimestamp(date, hour, minute)}`,
      hint: value
    };
  }

  return compactTechnicalId(value, 1);
}

export function compactFileReference(value: string | null | undefined, segments = 3) {
  if (!value) {
    return {
      value: "none",
      hint: undefined
    };
  }

  const normalized = value.replaceAll("\\", "/");
  const parts = normalized.split("/").filter(Boolean);
  if (parts.length <= segments) {
    return {
      value: normalized,
      hint: undefined
    };
  }

  return {
    value: parts.slice(-segments).join("/"),
    hint: normalized
  };
}
