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

export interface ConsoleDataSnapshot {
  assessment?: ConsoleReadyData["assessment"];
  assessmentHistory?: ConsoleReadyData["assessmentHistory"];
  posture?: ConsoleReadyData["posture"];
  method?: ConsoleReadyData["method"];
  audit?: ConsoleReadyData["audit"];
  overview?: ConsoleReadyData["overview"];
  indicators?: ConsoleReadyData["indicators"];
  events?: ConsoleReadyData["events"];
  sources?: ConsoleReadyData["sources"];
  backtests?: ConsoleReadyData["backtests"];
  backtestTimeline?: ConsoleReadyData["backtestTimeline"];
}

export function useConsoleData(requiredKeys: Array<keyof ConsoleReadyData>) {
  const queryClient = useQueryClient();
  const isRequired = (key: keyof ConsoleReadyData) => requiredKeys.includes(key);

  const assessment = useQuery({
    queryKey: ["assessment-current"],
    queryFn: api.assessmentCurrent,
    ...liveQueryOptions
  });
  const systemHealth = useQuery({
    queryKey: ["system-health"],
    queryFn: api.systemHealth,
    ...liveQueryOptions
  });
  const assessmentHistory = useQuery({
    queryKey: ["assessment-history"],
    queryFn: api.assessmentHistory,
    enabled: isRequired("assessmentHistory"),
    ...liveQueryOptions
  });
  const posture = useQuery({
    queryKey: ["assessment-posture"],
    queryFn: api.assessmentPosture,
    enabled: isRequired("posture"),
    ...liveQueryOptions
  });
  const method = useQuery({
    queryKey: ["assessment-method"],
    queryFn: api.assessmentMethod,
    enabled: isRequired("method"),
    ...liveQueryOptions
  });
  const audit = useQuery({
    queryKey: ["research-audit"],
    queryFn: api.researchAudit,
    enabled: isRequired("audit"),
    ...liveQueryOptions
  });
  const overview = useQuery({
    queryKey: ["overview"],
    queryFn: api.overview,
    enabled: isRequired("overview"),
    ...liveQueryOptions
  });
  const indicators = useQuery({
    queryKey: ["indicators"],
    queryFn: api.indicators,
    enabled: isRequired("indicators"),
    ...liveQueryOptions
  });
  const events = useQuery({
    queryKey: ["events-recent"],
    queryFn: api.eventsRecent,
    enabled: isRequired("events"),
    ...liveQueryOptions
  });
  const sources = useQuery({
    queryKey: ["sources"],
    queryFn: api.sources,
    enabled: isRequired("sources") || assessment.isSuccess,
    ...liveQueryOptions
  });
  const backtests = useQuery({
    queryKey: ["backtests"],
    queryFn: api.backtests,
    enabled: isRequired("backtests"),
    ...liveQueryOptions
  });
  const backtestTimeline = useQuery({
    queryKey: ["backtests-timeline"],
    queryFn: api.backtestTimeline,
    enabled: isRequired("backtestTimeline"),
    ...liveQueryOptions
  });
  const reload = useMutation({
    mutationFn: api.systemReload,
    onSuccess: async () => {
      await queryClient.invalidateQueries();
    }
  });

  const data: ConsoleDataSnapshot = {
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
  };

  const queries = {
    assessment,
    systemHealth,
    assessmentHistory,
    posture,
    method,
    audit,
    overview,
    indicators,
    events,
    sources,
    backtests,
    backtestTimeline
  };

  return {
    ...queries,
    data,
    queries,
    reload
  };
}
