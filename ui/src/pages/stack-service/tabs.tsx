import LogSection from "@/components/log-section";
import TerminalSection from "@/components/terminal/section";
import { usePermissions, useRead } from "@/lib/hooks";
import { useServer } from "@/resources/server";
import { ICONS } from "@/lib/icons";
import {
  ColorIntention,
  MobileFriendlyTabsSelector,
  TabNoContent,
} from "mogh_ui";
import { Tabs } from "@mantine/core";
import { useLocalStorage } from "@mantine/hooks";
import { Types } from "komodo_client";
import { useMemo, useState } from "react";
import StackServiceInspect from "./inspect";
import SwarmServiceTasksSection from "../swarm/service/tasks";

export type StackServiceTabsView = "Tasks" | "Log" | "Inspect" | "Terminals";

export interface StackServiceTabsProps {
  stack: Types.StackListItem;
  service: string;
  container: Types.ContainerListItem | undefined;
  swarmService: Types.SwarmServiceListItem | undefined;
  intention: ColorIntention;
}

export default function StackServiceTabs({
  stack,
  service,
  container,
  swarmService,
  intention,
}: StackServiceTabsProps) {
  const [_view, setView] = useLocalStorage<StackServiceTabsView>({
    key: `stack-${stack.id}-${service}-tabs-v2`,
    defaultValue: "Log",
  });
  const { specificLogs, specificInspect, specificTerminal } = usePermissions({
    type: "Stack",
    id: stack.id,
  });

  const down = !swarmService && !container;
  const isSwarm = !!swarmService && !!stack.info.swarm_id;

  const containerTerminalsDisabled =
    useServer(stack.info.server_id)?.info.container_terminals_disabled ?? false;

  const swarmTasks = useRead(
    "ListSwarmTasks",
    { swarm: stack.info.swarm_id },
    { enabled: isSwarm, refetchInterval: 10_000 },
  ).data;

  const runningTasks = useMemo(() => {
    if (!isSwarm || !swarmService?.ID) return [];
    return (swarmTasks ?? [])
      .filter(
        (t) =>
          t.ServiceID === swarmService.ID &&
          t.State === Types.TaskState.RUNNING &&
          !!t.ContainerID &&
          !!t.ID,
      )
      .sort((a, b) =>
        (b.UpdatedAt ?? "").localeCompare(a.UpdatedAt ?? ""),
      );
  }, [isSwarm, swarmService?.ID, swarmTasks]);

  const taskOptions = useMemo(
    () =>
      runningTasks.map((t) => ({
        id: t.ID!,
        label: `${t.ID!.slice(0, 12)}${t.NodeID ? ` @ ${t.NodeID.slice(0, 8)}` : ""}`,
      })),
    [runningTasks],
  );

  const logDisabled = !specificLogs || down;
  const inspectDisabled = !specificInspect || down;

  const composeTerminalDisabled =
    !specificTerminal ||
    containerTerminalsDisabled ||
    container?.state !== Types.ContainerStateStatusEnum.Running;

  const swarmTerminalDisabled =
    !specificTerminal || runningTasks.length === 0;

  const terminalDisabled = isSwarm
    ? swarmTerminalDisabled
    : composeTerminalDisabled;
  const terminalHidden = isSwarm
    ? !specificTerminal
    : !container || !specificTerminal;

  const view =
    (!stack.info.swarm_id && _view === "Tasks") ||
    (inspectDisabled && _view === "Inspect") ||
    (terminalDisabled && _view === "Terminals")
      ? "Log"
      : _view;

  const tabs = useMemo<TabNoContent[]>(
    () => [
      {
        value: "Tasks",
        hidden: !swarmService,
        icon: ICONS.SwarmTask,
      },
      {
        value: "Log",
        disabled: logDisabled,
        icon: ICONS.Log,
      },
      {
        value: "Inspect",
        disabled: inspectDisabled,
        icon: ICONS.Inspect,
      },
      {
        value: "Terminals",
        disabled: terminalDisabled,
        hidden: terminalHidden,
        icon: ICONS.Terminal,
      },
    ],
    [
      !!swarmService,
      logDisabled,
      inspectDisabled,
      terminalDisabled,
      terminalHidden,
    ],
  );

  const Selector = (
    <MobileFriendlyTabsSelector
      tabs={tabs}
      value={view}
      onValueChange={setView as any}
    />
  );

  const target: Types.TerminalTarget = useMemo(
    () => ({
      type: "Stack",
      params: {
        stack: stack.id,
        service,
        ...(taskOptions.length === 1
          ? { task: taskOptions[0].id }
          : {}),
      },
    }),
    [stack.id, service, taskOptions],
  );

  const _search = useState("");

  let View = Selector;
  switch (view) {
    case "Tasks":
      View = (
        <SwarmServiceTasksSection
          id={stack.info.swarm_id}
          serviceId={swarmService?.ID}
          titleOther={Selector}
          _search={_search}
        />
      );
      break;
    case "Log":
      View = (
        <LogSection
          target={{ type: "Stack", stackId: stack.id, services: [service] }}
          titleOther={Selector}
          disabled={logDisabled}
        />
      );
      break;
    case "Inspect":
      View = (
        <StackServiceInspect
          stackId={stack.id}
          service={service}
          useSwarm={!!swarmService}
          titleOther={Selector}
        />
      );
      break;
    case "Terminals":
      View = (
        <TerminalSection
          target={target}
          titleOther={Selector}
          tasks={isSwarm ? taskOptions : undefined}
        />
      );
      break;
  }

  return (
    <Tabs color={intention} value={view}>
      {View}
    </Tabs>
  );
}
