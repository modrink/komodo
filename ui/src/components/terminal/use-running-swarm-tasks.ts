import { useRead } from "@/lib/hooks";
import { TerminalTaskOption } from "@/components/terminal/new";
import { useMemo } from "react";

/** RUNNING swarm tasks for Stack Terminals picker (Stack Read — no Swarm Read). */
export function useRunningSwarmTaskOptions(args: {
  stackId: string | undefined;
  /** Limit to one compose service name (Stack Service page). */
  service?: string;
  enabled?: boolean;
}): TerminalTaskOption[] {
  const { stackId, service, enabled = !!stackId } = args;

  const tasks = useRead(
    "ListStackTerminalTasks",
    { stack: stackId ?? "", service },
    { enabled: enabled && !!stackId, refetchInterval: 10_000 },
  ).data;

  return useMemo(() => {
    if (!enabled || !stackId) return [];
    return (tasks ?? []).map((t) => ({
      id: t.id,
      service: t.service,
      label: `${t.service} · ${t.id.slice(0, 12)}${
        t.node_id ? ` @ ${t.node_id.slice(0, 8)}` : ""
      }`,
    }));
  }, [enabled, stackId, tasks]);
}
