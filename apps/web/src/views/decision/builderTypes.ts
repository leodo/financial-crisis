export interface DecisionRuntimeNotice {
  tone: "notice" | "notice error";
  title: string;
  body: string;
}

export interface DecisionRuntimeCard {
  label: string;
  value: string;
  detail: string;
}

export interface DecisionKeyIndicatorRow {
  id: string;
  title: string;
  detail: string;
  note: string;
}

export interface DecisionScoreBandRow {
  label: string;
  rangeText: string;
  note: string;
  active: boolean;
}

export interface DecisionSignalLayerRowModel {
  id: string;
  title: string;
  description: string;
  value: string;
  detail: string;
}

export interface DecisionAnalogRow {
  id: string;
  title: string;
  detail: string;
  score: string;
}

export interface DecisionRollingAuditEpisodeRow {
  key: string;
  classificationClass: string;
  classificationLabel: string;
  interval: string;
  duration: string;
  signalCount: string;
  note: string;
}
