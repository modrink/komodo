import { useLocalStorage } from "@mantine/hooks";
import { useStack } from ".";
import { usePermissions, useRead } from "@/lib/hooks";
import { Types } from "komodo_client";
import { useMemo } from "react";
import { MobileFriendlyTabsSelector, TabNoContent } from "mogh_ui";
import { Tabs } from "@mantine/core";
import { ICONS } from "@/lib/icons";
import { stackStateIntention } from "@/lib/color";
import { useServer } from "@/resources/server";
import StackConfig from "./config";
import StackInfo from "./info";
import StackServices from "./services";
import StackLog from "./log";
import TerminalSection from "@/components/terminal/section";
import { TerminalTaskOption } from "@/components/terminal/new";

type StackTabsView = "Config" | "Info" | "Services" | "Log" | "Terminals";

export default function StackTabs({ id }: { id: string }) {
  const [_view, setView] = useLocalStorage<StackTabsView>({
    key: `stack-${id}-tab-v2`,
    defaultValue: "Config",
  });
  const info = useStack(id)?.info;
  const { specificLogs, specificTerminal } = usePermissions({
    type: "Stack",
    id,
  });

  const services = useRead("ListStackServices", { stack: id }).data;
  const swarmStack = !!info?.swarm_id;

  const swarmTasks = useRead(
    "ListSwarmTasks",
    { swarm: info?.swarm_id ?? "" },
    { enabled: swarmStack, refetchInterval: 10_000 },
  ).data;

  const swarmTaskOptions: TerminalTaskOption[] = useMemo(() => {
    if (!swarmStack || !services) return [];
    const bySwarmServiceId = new Map(
      services
        .filter((s) => s.swarm_service?.ID)
        .map((s) => [s.swarm_service!.ID!, s.service] as const),
    );
    return (swarmTasks ?? [])
      .filter(
        (t) =>
          t.State === Types.TaskState.RUNNING &&
          !!t.ContainerID &&
          !!t.ID &&
          !!t.ServiceID &&
          bySwarmServiceId.has(t.ServiceID),
      )
      .sort((a, b) => (b.UpdatedAt ?? "").localeCompare(a.UpdatedAt ?? ""))
      .map((t) => ({
        id: t.ID!,
        service: bySwarmServiceId.get(t.ServiceID!)!,
        label: `${bySwarmServiceId.get(t.ServiceID!)} · ${t.ID!.slice(0, 12)}`,
      }));
  }, [swarmStack, services, swarmTasks]);

  const containerTerminalsDisabled =
    useServer(info?.server_id)?.info.container_terminals_disabled ?? false;

  const state = info?.state;
  const hideInfo = !info?.files_on_host && !info?.repo && !info?.linked_repo;
  const hideLogs =
    state === undefined ||
    state === Types.StackState.Unknown ||
    state === Types.StackState.Down ||
    !specificLogs;

  const composeTerminalDisabled =
    !specificTerminal ||
    containerTerminalsDisabled ||
    services?.every(
      (service) =>
        !service.container ||
        service.container.state !== Types.ContainerStateStatusEnum.Running,
    );

  const swarmTerminalDisabled =
    !specificTerminal || swarmTaskOptions.length === 0;

  const terminalDisabled = swarmStack
    ? swarmTerminalDisabled
    : !!composeTerminalDisabled;

  const view =
    (_view === "Info" && hideInfo) ||
    (_view === "Terminals" && terminalDisabled) ||
    (_view === "Log" && hideLogs)
      ? "Config"
      : _view;

  const tabs = useMemo<TabNoContent[]>(
    () => [
      {
        value: "Config",
        icon: ICONS.Settings,
      },
      {
        value: "Info",
        hidden: hideInfo,
        icon: ICONS.Search,
      },
      {
        value: "Services",
        icon: ICONS.Service,
      },
      {
        value: "Log",
        disabled: hideLogs,
        icon: ICONS.Log,
      },
      {
        value: "Terminals",
        disabled: terminalDisabled,
        hidden: !specificTerminal,
        icon: ICONS.Terminal,
      },
    ],
    [hideInfo, specificLogs, hideLogs, specificTerminal, terminalDisabled],
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
        stack: id,
      },
    }),
    [id],
  );

  let View = Selector;
  switch (view) {
    case "Config":
      View = <StackConfig id={id} titleOther={Selector} />;
      break;
    case "Info":
      View = <StackInfo id={id} titleOther={Selector} />;
      break;
    case "Services":
      View = <StackServices id={id} titleOther={Selector} />;
      break;
    case "Log":
      View = <StackLog id={id} titleOther={Selector} />;
      break;
    case "Terminals":
      View = (
        <TerminalSection
          target={target}
          services={services?.map((s) => s.service)}
          tasks={swarmStack ? swarmTaskOptions : undefined}
          titleOther={Selector}
        />
      );
      break;
  }

  return (
    <Tabs
      color={stackStateIntention(
        state,
        info?.services &&
          !info.services.every((service) => !service.update_available),
      )}
      value={view}
    >
      {View}
    </Tabs>
  );
}
