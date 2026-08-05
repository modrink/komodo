# Terminals

Komodo provides browser-based terminal sessions for servers and containers. Sessions are persistent,
support multiple simultaneous connections, and commands can be scripted and scheduled.

## Server Terminals

Open a shell directly on a connected server. The default command is `bash`, configurable per-Periphery via `default_terminal_command`.

## Container Terminals

Connect to a running container in two modes:

- **Exec** (default) — runs a new command inside the container (`docker exec`). Typically used for interactive shells.
- **Attach** — attaches to the container's main process (`docker attach`). Useful for interacting with the primary process directly.

Container terminals are available on **Deployments**, **Stack services**, and any container visible on a server.

## Swarm Task Terminals

Open an exec shell into a **running Swarm task** container (same idea as Portainer's task console).

**Preferred (reuses Stack Terminals):** on a Swarm-attached **Stack** or **Stack Service**, open the existing **Terminals** tab → **New**. Flow matches compose Stacks, with an extra step when needed:

1. Pick **service** (Stack page only)
2. Pick **task** if multiple RUNNING replicas (skipped when only one)
3. Pick **command** (`sh` / `bash` / …)

Core resolves the task’s node → Komodo Server (hostname match) → `docker exec` on that Periphery. Terminals stay `TerminalTarget::Stack` so list/delete/reconnect keep working.

**Also:** Swarm **Task** page → **Terminals** tab, and the terminal action icon on task tables (`TerminalTarget::SwarmTask`).

- Requires **`Terminal`** on the **Stack** (Stack Terminals path — create/list/delete/reconnect; picker uses Stack **Read** via `ListStackTerminalTasks`) or on the **Swarm** (Task page path).
- The node hosting the task must be a Komodo **Server** with Periphery.
- If no matching Server is found, terminal creation fails with a clear error — add the worker as a Server first.

## Multiple Sessions

You can create multiple named terminal sessions on the same resource. Each session has its own independent PTY process and output history.

- Terminal names must be unique within a target (e.g. two terminals named "debug" can exist on different servers).
- Multiple users can connect to the same terminal session simultaneously — output is broadcast to all connected clients.
- Sessions persist until explicitly deleted or Periphery restarts.

## Terminal History

Each terminal maintains a rolling 1 MiB output buffer. When you reconnect to an existing session, the history is replayed so you can see previous output.

## CLI

Terminal sessions can also be accessed from the command line using the [Komodo CLI](./ecosystem/cli.mdx#terminals).

- `km ssh <server>` — open a shell on a server
- `km exec <container> <shell>` — exec into a container
- `km attach <container>` — attach to a container's main process

Press **Alt+Q** to disconnect from any CLI terminal session while the session itself stays running.

## Execute Terminal

The `execute_terminal` API method allows you to run a command on a terminal and stream the output back over HTTP. This is useful for:

- **Actions** — TypeScript scripts can call `execute_terminal` on the Komodo client to run commands on any server or container and process the output programmatically.
- **Automation** — integrate terminal command execution into external tools via the REST API.

The TypeScript client provides convenience methods for each target type. All methods accept optional `callbacks` with `onLine` (called per output line) and `onFinish` (called with the exit code).

```typescript
// Server terminal
await komodo.execute_server_terminal({
  server: "my-server",
  terminal: "automation",
  command: "df -h",
  init: { command: "bash", recreate: "DifferentCommand" },
}, {
  onLine: (line) => console.log(line),
  onFinish: (code) => console.log("Exit code:", code),
});

// Container terminal
await komodo.execute_container_terminal({
  server: "my-server",
  container: "my-container",
  terminal: "debug",
  command: "cat /var/log/errors.log",
  init: { command: "sh", mode: "Exec", recreate: "Never" },
});

// Stack service terminal
await komodo.execute_stack_service_terminal({
  stack: "my-stack",
  service: "web",
  terminal: "debug",
  command: "nginx -t",
  init: { command: "sh" },
});

// Deployment terminal
await komodo.execute_deployment_terminal({
  deployment: "my-deployment",
  terminal: "check",
  command: "node --version",
  init: { command: "sh", recreate: "Always" },
});

// Swarm task terminal
await komodo.execute_swarm_task_terminal({
  swarm: "production-swarm",
  task: "task-id",
  terminal: "debug",
  command: "env",
  init: { command: "sh" },
});
```

## Periphery Configuration

Terminal behavior can be configured in the Periphery config file:

| Setting | Description | Default |
|---|---|---|
| `default_terminal_command` | Default shell command for new server terminals. | `bash` |
| `disable_terminals` | Disable server terminal sessions. | `false` |
| `disable_container_terminals` | Disable container terminal sessions. | `false` |
