import { useWrite } from "@/lib/hooks";
import { filterBySplit } from "mogh_ui";
import { ICONS } from "@/lib/icons";
import {
  Button,
  ButtonProps,
  Combobox,
  ComboboxProps,
  Divider,
  Group,
  Text,
} from "@mantine/core";
import { useLocalStorage } from "@mantine/hooks";
import { notifications } from "@mantine/notifications";
import { Types } from "komodo_client";
import { useState } from "react";
import { useSearchCombobox } from "mogh_ui";

export type TerminalTaskOption = {
  id: string;
  label: string;
  /** Compose service name — used to filter tasks after service pick on Stack page */
  service?: string;
};

export interface NewTerminalProps extends ComboboxProps {
  target: Types.TerminalTarget;
  existingTerminals: string[] | undefined;
  refetchTerminals: () => void;
  setSelected: (value: { selected: string | undefined }) => void;
  /** Stack compose: pick service then command */
  services?: string[];
  /**
   * Swarm Stack adapt: pick RUNNING task then command.
   * If exactly one option, skipped automatically.
   */
  tasks?: TerminalTaskOption[];
  targetProps?: ButtonProps;
}

const BASE_COMMANDS = ["sh", "bash"];

export default function NewTerminal({
  target,
  existingTerminals,
  refetchTerminals,
  setSelected,
  services,
  tasks,
  position = "bottom-start",
  targetProps,
  ...comboboxProps
}: NewTerminalProps) {
  const [service, setService] = useState<string | undefined>(undefined);
  const [task, setTask] = useState<string | undefined>(
    tasks?.length === 1 ? tasks[0].id : undefined,
  );
  const { mutateAsync: createTerminal } = useWrite("CreateTerminal", {
    onSuccess: () =>
      notifications.show({ message: "Terminal created.", color: "green" }),
  });

  const { search, setSearch, combobox } = useSearchCombobox();

  const needsService = !!services && !service;
  const needsTask =
    !!tasks &&
    tasks.filter((t) => !service || !t.service || t.service === service)
      .length > 1 &&
    !task &&
    (!services || !!service);

  const tasksForService = tasks?.filter(
    (t) => !service || !t.service || t.service === service,
  );

  const create = async (command: string | undefined, isServer: boolean) => {
    if (!existingTerminals) return;
    const pickedTask =
      task ??
      (tasksForService?.length === 1 ? tasksForService[0].id : undefined);
    const name = nextTerminalName(
      command,
      service,
      pickedTask,
      existingTerminals,
    );
    const nextTarget =
      target.type === "Stack"
        ? {
            type: "Stack" as const,
            params: {
              ...target.params,
              ...(service ? { service } : {}),
              ...(pickedTask ? { task: pickedTask } : {}),
            },
          }
        : service
          ? { ...target, params: { ...target.params, service } as any }
          : target;
    await createTerminal({
      target: nextTarget,
      name,
      command,
      mode:
        !isServer && !command
          ? Types.ContainerTerminalMode.Attach
          : Types.ContainerTerminalMode.Exec,
    });
    refetchTerminals();
    setTimeout(() => {
      setSelected({
        selected: name,
      });
    }, 100);
  };

  const isServer = target.type === "Server";

  const [commands, setCommands] = useLocalStorage({
    key: isServer ? "server-commands-v2" : "container-commands-v2",
    defaultValue: isServer ? BASE_COMMANDS : [...BASE_COMMANDS, "attach"],
  });
  const filtered = filterBySplit(commands, search, (item) => item);

  const placeholder = needsService
    ? "Select Service"
    : needsTask
      ? "Select Task"
      : "Select Command";

  return (
    <Combobox
      store={combobox}
      width={300}
      position={position}
      onOptionSubmit={(command) => {
        if (needsService) {
          setService(command);
          const forSvc = tasks?.filter(
            (t) => !t.service || t.service === command,
          );
          if (forSvc?.length === 1) {
            setTask(forSvc[0].id);
          } else {
            setTask(undefined);
          }
          return;
        }
        if (needsTask) {
          setTask(command);
          return;
        }
        create(
          command === "Default" || (!isServer && command === "attach")
            ? undefined
            : command === "Custom"
              ? search
              : command,
          isServer,
        ).then(() => {
          combobox.closeDropdown();
          setService(undefined);
          setTask(tasks?.length === 1 ? tasks[0].id : undefined);
        });
      }}
      onClose={() => {
        setService(undefined);
        setTask(tasks?.length === 1 ? tasks[0].id : undefined);
      }}
      {...comboboxProps}
    >
      <Combobox.Target>
        <Button
          leftSection={<ICONS.Create size="1rem" />}
          onClick={() => combobox.toggleDropdown()}
          {...targetProps}
        >
          New
        </Button>
      </Combobox.Target>
      <Combobox.Dropdown>
        <Combobox.Search
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          leftSection={<ICONS.Search size="1rem" style={{ marginRight: 6 }} />}
          placeholder={placeholder}
        />
        <Combobox.Options mah={224} style={{ overflowY: "auto" }}>
          {needsService &&
            services!.map((svc) => (
              <Combobox.Option key={svc} value={svc}>
                <Text>{svc}</Text>
              </Combobox.Option>
            ))}
          {needsTask &&
            tasksForService!.map((t) => (
              <Combobox.Option key={t.id} value={t.id}>
                <Text>{t.label}</Text>
              </Combobox.Option>
            ))}
          {!needsService && !needsTask && (
            <>
              {isServer && !search && (
                <Combobox.Option value="Default">Default</Combobox.Option>
              )}
              {filtered.map((command) => (
                <Combobox.Option key={command} value={command}>
                  <Text>{command}</Text>
                </Combobox.Option>
              ))}

              <Divider />

              <Combobox.Option
                value="Custom"
                disabled={!search || commands.includes(search)}
                onSelect={() => setCommands((c) => [...c, search])}
              >
                <Group justify="center" gap="xs">
                  <ICONS.Create size="1rem" />
                  Custom
                </Group>
              </Combobox.Option>
            </>
          )}
        </Combobox.Options>
      </Combobox.Dropdown>
    </Combobox>
  );
}

function nextTerminalName(
  __command: string | undefined,
  service: string | undefined,
  task: string | undefined,
  existingTerminals: string[],
) {
  const _command = !__command ? "attach" : __command.split(" ")[0];
  const taskShort = task ? task.slice(0, 8) : undefined;
  const command = `${service ? service + " " : ""}${taskShort ? taskShort + " " : ""}${_command}`;
  for (let i = 1; i <= existingTerminals.length + 1; i++) {
    const name = i > 1 ? `${command} ${i}` : command;
    if (!existingTerminals.includes(name)) {
      return name;
    }
  }
  return command;
}
