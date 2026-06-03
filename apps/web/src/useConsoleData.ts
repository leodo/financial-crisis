import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./api";

const liveQueryOptions = {
  refetchInterval: 60_000,
  refetchIntervalInBackground: true,
  refetchOnMount: "always" as const,
  refetchOnWindowFocus: true,
  staleTime: 10_000
};

export interface ConsoleReadyData {
  assessment: Awaited<ReturnType<typeof api.assessmentCurrent>>;
  assessmentHistory: Awaited<ReturnType<typeof api.assessmentHistory>>;
  posture: Awaited<ReturnType<typeof api.assessmentPosture>>;
  method: Awaited<ReturnType<typeof api.assessmentMethod>>;
  audit: Awaited<ReturnType<typeof api.researchAudit>>;
  overview: Awaited<ReturnType<typeof api.overview>>;
  indicators: Awaited<ReturnType<typeof api.indicators>>;
  events: Awaited<ReturnType<typeof api.eventsRecent>>;
  sources: Awaited<ReturnType<typeof api.sources>>;
  backtests: Awaited<ReturnType<typeof api.backtests>>;
  backtestTimeline: Awaited<ReturnType<typeof api.backtestTimeline>>;
}

export function useConsoleData() {
  const queryClient = useQueryClient();

  const assessment = useQuery({
    queryKey: ["assessment-current"],
    queryFn: api.assessmentCurrent,
    ...liveQueryOptions
  });
  const assessmentHistory = useQuery({
    queryKey: ["assessment-history"],
    queryFn: api.assessmentHistory,
    ...liveQueryOptions
  });
  const posture = useQuery({
    queryKey: ["assessment-posture"],
    queryFn: api.assessmentPosture,
    ...liveQueryOptions
  });
  const method = useQuery({
    queryKey: ["assessment-method"],
    queryFn: api.assessmentMethod,
    ...liveQueryOptions
  });
  const audit = useQuery({
    queryKey: ["research-audit"],
    queryFn: api.researchAudit,
    ...liveQueryOptions
  });
  const overview = useQuery({ queryKey: ["overview"], queryFn: api.overview, ...liveQueryOptions });
  const indicators = useQuery({
    queryKey: ["indicators"],
    queryFn: api.indicators,
    ...liveQueryOptions
  });
  const events = useQuery({
    queryKey: ["events-recent"],
    queryFn: api.eventsRecent,
    ...liveQueryOptions
  });
  const sources = useQuery({ queryKey: ["sources"], queryFn: api.sources, ...liveQueryOptions });
  const backtests = useQuery({
    queryKey: ["backtests"],
    queryFn: api.backtests,
    ...liveQueryOptions
  });
  const backtestTimeline = useQuery({
    queryKey: ["backtests-timeline"],
    queryFn: api.backtestTimeline,
    ...liveQueryOptions
  });
  const reload = useMutation({
    mutationFn: api.systemReload,
    onSuccess: async () => {
      await queryClient.invalidateQueries();
    }
  });

  const isLoading =
    assessment.isLoading ||
    assessmentHistory.isLoading ||
    posture.isLoading ||
    method.isLoading ||
    audit.isLoading ||
    overview.isLoading ||
    indicators.isLoading ||
    events.isLoading ||
    sources.isLoading ||
    backtests.isLoading ||
    backtestTimeline.isLoading;
  const error =
    assessment.error ??
    assessmentHistory.error ??
    posture.error ??
    method.error ??
    audit.error ??
    overview.error ??
    indicators.error ??
    events.error ??
    sources.error ??
    backtests.error ??
    backtestTimeline.error;

  const readyData: ConsoleReadyData | null =
    !isLoading &&
    !error &&
    assessment.data &&
    assessmentHistory.data &&
    posture.data &&
    method.data &&
    audit.data &&
    overview.data &&
    indicators.data &&
    events.data &&
    sources.data &&
    backtests.data &&
    backtestTimeline.data
      ? {
          assessment: assessment.data,
          assessmentHistory: assessmentHistory.data,
          posture: posture.data,
          method: method.data,
          audit: audit.data,
          overview: overview.data,
          indicators: indicators.data,
          events: events.data,
          sources: sources.data,
          backtests: backtests.data,
          backtestTimeline: backtestTimeline.data
        }
      : null;

  return {
    assessment,
    assessmentHistory,
    posture,
    method,
    audit,
    overview,
    indicators,
    events,
    sources,
    backtests,
    backtestTimeline,
    reload,
    isLoading,
    error,
    readyData
  };
}
