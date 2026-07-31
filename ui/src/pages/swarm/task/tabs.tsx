import LogSection from "@/components/log-section";
import TerminalSection from "@/components/terminal/section";
import { usePermissions } from "@/lib/hooks";
import { ICONS } from "@/lib/icons";
import { ColorIntention, MobileFriendlyTabsSelector } from "mogh_ui";
import { Tabs } from "@mantine/core";
import { useLocalStorage } from "@mantine/hooks";
import { Types } from "komodo_client";
import { useMemo } from "react";
import InspectSection from "@/components/inspect-section";

type SwarmTaskTabsView = "Log" | "Inspect" | "Terminals";

export interface SwarmTaskTabsProps {
  swarm: Types.SwarmListItem;
  task: string;
  taskItem: Types.SwarmTaskListItem | undefined;
  intent: ColorIntention;
}

export default function SwarmTaskTabs({
  swarm,
  task,
  taskItem,
  intent,
}: SwarmTaskTabsProps) {
  const [_view, setView] = useLocalStorage<SwarmTaskTabsView>({
    key: `swarm-${swarm.id}-task-${task}-tabs-v3`,
    defaultValue: "Log",
  });
  const { specificLogs, specificInspect, specificTerminal } = usePermissions({
    type: "Swarm",
    id: swarm.id,
  });

  const terminalTarget: Types.TerminalTarget = useMemo(
    () => ({
      type: "SwarmTask",
      params: { swarm: swarm.id, task: taskItem?.ID ?? task },
    }),
    [swarm.id, task, taskItem?.ID],
  );

  const terminalDisabled =
    !specificTerminal ||
    taskItem?.State !== Types.TaskState.RUNNING ||
    !taskItem?.ContainerID;

  const view =
    (!specificInspect && _view === "Inspect") ||
    (terminalDisabled && _view === "Terminals")
      ? "Log"
      : _view;

  const tabs = useMemo(
    () => [
      {
        value: "Log",
        icon: ICONS.Log,
        disabled: !specificLogs,
      },
      {
        value: "Inspect",
        icon: ICONS.Inspect,
        disabled: !specificInspect,
      },
      {
        value: "Terminals",
        icon: ICONS.Terminal,
        disabled: terminalDisabled,
        hidden: !specificTerminal,
      },
    ],
    [specificLogs, specificInspect, specificTerminal, terminalDisabled],
  );

  const Selector = (
    <MobileFriendlyTabsSelector
      tabs={tabs}
      value={view}
      onValueChange={setView as any}
    />
  );

  let View = Selector;
  switch (view) {
    case "Log":
      View = (
        <LogSection
          target={{ type: "SwarmService", swarmId: swarm.id, service: task }}
          titleOther={Selector}
          disabled={!specificLogs}
        />
      );
      break;
    case "Inspect":
      View = (
        <InspectSection
          request={{
            type: "InspectSwarmTask",
            params: { swarm: swarm.id, task },
          }}
          titleOther={Selector}
        />
      );
      break;
    case "Terminals":
      View = (
        <TerminalSection target={terminalTarget} titleOther={Selector} />
      );
      break;
  }

  return (
    <Tabs color={intent} value={view} mt="lg">
      {View}
    </Tabs>
  );
}
