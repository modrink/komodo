use anyhow::{Context as _, anyhow};
use komodo_client::{
  api::{terminal::InitTerminal, write::CreateTerminal},
  entities::{
    deployment::Deployment,
    permission::PermissionLevel,
    server::{Server, ServerState},
    stack::Stack,
    swarm::Swarm,
    terminal::{ContainerTerminalMode, Terminal, TerminalTarget},
    user::User,
  },
};
use periphery_client::api;

use crate::{
  helpers::{periphery_client, swarm::swarm_request, swarm_terminal},
  periphery::PeripheryClient,
  permission::get_check_permissions,
  resource,
  state::{
    server_status_cache, stack_status_cache, swarm_status_cache,
  },
};

pub async fn setup_target_for_user(
  target: TerminalTarget,
  terminal: Option<String>,
  init: Option<InitTerminal>,
  user: &User,
) -> anyhow::Result<(TerminalTarget, String, PeripheryClient)> {
  match target {
    TerminalTarget::Server { server } => {
      setup_server_target_for_user(
        server.context("Missing 'target.params.server'")?,
        terminal,
        init,
        user,
      )
      .await
    }
    TerminalTarget::Container { server, container } => {
      setup_container_target_for_user(
        server, container, terminal, init, user,
      )
      .await
    }
    TerminalTarget::Stack { stack, service } => {
      setup_stack_service_target_for_user(
        stack,
        service.context("Missing 'target.params.service'")?,
        terminal,
        init,
        user,
      )
      .await
    }
    TerminalTarget::Deployment { deployment } => {
      setup_deployment_target_for_user(
        deployment, terminal, init, user,
      )
      .await
    }
    TerminalTarget::SwarmTask { swarm, task } => {
      setup_swarm_task_target_for_user(
        swarm, task, terminal, init, user,
      )
      .await
    }
  }
}

async fn setup_server_target_for_user(
  server: String,
  terminal: Option<String>,
  init: Option<InitTerminal>,
  user: &User,
) -> anyhow::Result<(TerminalTarget, String, PeripheryClient)> {
  let server = get_check_permissions::<Server>(
    &server,
    user,
    PermissionLevel::Read.terminal(),
  )
  .await?;

  let terminal = terminal.unwrap_or_else(|| {
    init
      .as_ref()
      .and_then(|init| init.command.clone())
      .unwrap_or_else(|| String::from("term"))
  });

  let periphery = periphery_client(&server).await?;

  if let Some(init) = init {
    periphery
      .request(api::terminal::CreateServerTerminal {
        name: Some(terminal.clone()),
        command: init.command,
        recreate: init.recreate,
      })
      .await
      .context("Failed to create Server Terminal on Periphery")?;
  }

  Ok((
    TerminalTarget::Server {
      server: Some(server.id),
    },
    terminal,
    periphery,
  ))
}

async fn setup_container_target_for_user(
  server: String,
  container: String,
  terminal: Option<String>,
  init: Option<InitTerminal>,
  user: &User,
) -> anyhow::Result<(TerminalTarget, String, PeripheryClient)> {
  let server = get_check_permissions::<Server>(
    &server,
    user,
    PermissionLevel::Read.terminal(),
  )
  .await?;

  let terminal = default_container_terminal_name(
    terminal,
    &container,
    init.as_ref(),
  );

  let periphery = periphery_client(&server).await?;

  let target = TerminalTarget::Container {
    server: server.id,
    container: container.clone(),
  };

  if let Some(init) = init {
    create_container_terminal_inner(
      CreateTerminal {
        name: Some(terminal.clone()),
        target: target.clone(),
        command: init.command,
        mode: init.mode,
        recreate: init.recreate,
      },
      &periphery,
      container,
    )
    .await?;
  }

  Ok((target, terminal, periphery))
}

async fn setup_stack_service_target_for_user(
  stack: String,
  service: String,
  terminal: Option<String>,
  init: Option<InitTerminal>,
  user: &User,
) -> anyhow::Result<(TerminalTarget, String, PeripheryClient)> {
  let (target, periphery, container) =
    get_stack_service_periphery_container(&stack, &service, user)
      .await?;

  let terminal = default_container_terminal_name(
    terminal,
    &container,
    init.as_ref(),
  );

  if let Some(init) = init {
    create_container_terminal_inner(
      CreateTerminal {
        name: Some(terminal.clone()),
        target: target.clone(),
        command: init.command,
        mode: init.mode,
        recreate: init.recreate,
      },
      &periphery,
      container,
    )
    .await?;
  }

  Ok((target, terminal, periphery))
}

async fn setup_deployment_target_for_user(
  deployment: String,
  terminal: Option<String>,
  init: Option<InitTerminal>,
  user: &User,
) -> anyhow::Result<(TerminalTarget, String, PeripheryClient)> {
  let (target, periphery, container) =
    get_deployment_periphery_container(&deployment, user).await?;

  let terminal = default_container_terminal_name(
    terminal,
    &container,
    init.as_ref(),
  );

  if let Some(init) = init {
    create_container_terminal_inner(
      CreateTerminal {
        name: Some(terminal.clone()),
        target: target.clone(),
        command: init.command,
        mode: init.mode,
        recreate: init.recreate,
      },
      &periphery,
      container,
    )
    .await?;
  }

  Ok((target, terminal, periphery))
}

async fn setup_swarm_task_target_for_user(
  swarm: String,
  task: String,
  terminal: Option<String>,
  init: Option<InitTerminal>,
  user: &User,
) -> anyhow::Result<(TerminalTarget, String, PeripheryClient)> {
  let (target, periphery, container) =
    get_swarm_task_periphery_container(&swarm, &task, user, true)
      .await?;

  let terminal = default_container_terminal_name(
    terminal,
    &container,
    init.as_ref(),
  );

  if let Some(init) = init {
    create_container_terminal_inner(
      CreateTerminal {
        name: Some(terminal.clone()),
        target: target.clone(),
        command: init.command,
        mode: init.mode,
        recreate: init.recreate,
      },
      &periphery,
      container,
    )
    .await?;
  }

  Ok((target, terminal, periphery))
}

fn default_container_terminal_name(
  terminal: Option<String>,
  container: &str,
  init: Option<&InitTerminal>,
) -> String {
  terminal.unwrap_or_else(|| {
    init
      .as_ref()
      .map(|init| {
        init.command.clone().unwrap_or_else(|| {
          init.mode.unwrap_or_default().as_ref().to_string()
        })
      })
      .unwrap_or_else(|| container.to_string())
  })
}

pub async fn create_container_terminal_inner(
  CreateTerminal {
    name,
    target,
    command,
    mode,
    recreate,
  }: CreateTerminal,
  periphery: &PeripheryClient,
  container: String,
) -> anyhow::Result<Terminal> {
  match mode.unwrap_or_default() {
    ContainerTerminalMode::Exec => periphery
      .request(periphery_client::api::terminal::CreateContainerExecTerminal {
        name,
        target,
        container,
        command,
        recreate,
      })
      .await
      .context(
        "Failed to create Container Exec Terminal on Periphery",
      ),
    ContainerTerminalMode::Attach => periphery
      .request(periphery_client::api::terminal::CreateContainerAttachTerminal {
        name,
        target,
        container,
        recreate,
      })
      .await
      .context(
        "Failed to create Container Attach Terminal on Periphery",
      ),
  }
}

pub async fn get_stack_service_periphery_container(
  stack: &str,
  service: &str,
  user: &User,
) -> anyhow::Result<(TerminalTarget, PeripheryClient, String)> {
  let stack = get_check_permissions::<Stack>(
    stack,
    user,
    PermissionLevel::Read.terminal(),
  )
  .await?;

  let server =
    resource::get::<Server>(&stack.config.server_id).await?;

  let Some(status) = stack_status_cache().get(&stack.id).await else {
    return Err(anyhow!("Could not get Stack status"));
  };

  let container = status
    .curr
    .services
    .iter()
    .find(|s| s.service.as_str() == service)
    .with_context(|| {
      format!("Did not find Stack service matching {service}")
    })?
    .container
    .as_ref()
    .with_context(|| {
      format!("Did not find container for Stack service {service}")
    })?
    .name
    .clone();

  let periphery = periphery_client(&server).await?;

  Ok((
    TerminalTarget::Stack {
      stack: stack.id,
      service: Some(service.to_string()),
    },
    periphery,
    container,
  ))
}

pub async fn get_deployment_periphery_container(
  deployment: &str,
  user: &User,
) -> anyhow::Result<(TerminalTarget, PeripheryClient, String)> {
  let deployment = get_check_permissions::<Deployment>(
    deployment,
    user,
    PermissionLevel::Read.terminal(),
  )
  .await?;

  let server =
    resource::get::<Server>(&deployment.config.server_id).await?;

  let periphery = periphery_client(&server).await?;

  let container = deployment.deployed_name().to_string();

  Ok((
    TerminalTarget::Deployment {
      deployment: deployment.id,
    },
    periphery,
    container,
  ))
}

/// Resolve a Swarm task to the Periphery client on the node hosting its container.
///
/// When `require_running` is true, the task must be in `running` state with a container id
/// (used for create / connect). When false, still requires node + hostname match so
/// existing terminals can be listed / deleted after the task stops.
pub async fn get_swarm_task_periphery_container(
  swarm: &str,
  task: &str,
  user: &User,
  require_running: bool,
) -> anyhow::Result<(TerminalTarget, PeripheryClient, String)> {
  let swarm = get_check_permissions::<Swarm>(
    swarm,
    user,
    PermissionLevel::Read.terminal(),
  )
  .await?;

  let inspected = swarm_request(
    &swarm.config.server_ids,
    periphery_client::api::swarm::InspectSwarmTask {
      task: task.to_string(),
    },
  )
  .await
  .with_context(|| format!("Failed to inspect Swarm task {task}"))?;

  let task_id = inspected
    .id
    .clone()
    .context("Swarm task inspect missing task ID")?;

  let node_id = inspected
    .node_id
    .as_deref()
    .context("Swarm task has no assigned node")?;

  let container_id = inspected
    .status
    .as_ref()
    .and_then(|s| s.container_status.as_ref())
    .and_then(|c| c.container_id.clone());

  let state = inspected.status.as_ref().and_then(|s| s.state);

  swarm_terminal::check_swarm_task_running_gate(
    &task_id,
    state,
    container_id.as_deref(),
    require_running,
  )?;

  let container = container_id.unwrap_or_default();

  let node_candidates = resolve_swarm_node_identity_candidates(
    &swarm.id,
    &swarm.config.server_ids,
    node_id,
  )
  .await
  .with_context(|| {
    format!("Failed to resolve identity for Swarm node {node_id}")
  })?;

  let server =
    find_server_for_swarm_node_identities(&node_candidates).await?;

  if let Some(status) = server_status_cache().get(&server.id).await {
    if status.state != ServerState::Ok {
      return Err(anyhow!(
        "Server {} ({}) hosting Swarm task is not connected",
        server.name,
        server.id
      ));
    }
    if status
      .periphery_info
      .as_ref()
      .map(|i| i.container_terminals_disabled)
      .unwrap_or(false)
    {
      return Err(anyhow!(
        "Container terminals are disabled on Server {} ({})",
        server.name,
        server.id
      ));
    }
  }

  let periphery = periphery_client(&server).await?;

  Ok((
    TerminalTarget::SwarmTask {
      swarm: swarm.id,
      task: task_id,
    },
    periphery,
    container,
  ))
}

async fn resolve_swarm_node_identity_candidates(
  swarm_id: &str,
  manager_server_ids: &[String],
  node_id: &str,
) -> anyhow::Result<Vec<String>> {
  let cache = swarm_status_cache()
    .get_or_insert_default(&swarm_id.to_string())
    .await;
  let nodes = cache
    .lists
    .as_ref()
    .map(|l| l.nodes.as_slice())
    .unwrap_or(&[]);

  if let Some(node) =
    nodes.iter().find(|n| n.id.as_deref() == Some(node_id))
  {
    return swarm_terminal::require_node_candidates(
      node_id,
      swarm_terminal::node_identity_candidates_from_list_item(node),
    );
  }

  // Cache miss: inspect node via manager Periphery
  let inspected = swarm_request(
    manager_server_ids,
    periphery_client::api::swarm::InspectSwarmNode {
      node: node_id.to_string(),
    },
  )
  .await
  .with_context(|| {
    format!(
      "Swarm node {node_id} not in status cache and InspectSwarmNode failed"
    )
  })?;

  swarm_terminal::require_node_candidates(
    node_id,
    swarm_terminal::node_identity_candidates_from_inspect(&inspected),
  )
}

async fn find_server_for_swarm_node_identities(
  node_candidates: &[String],
) -> anyhow::Result<Server> {
  let servers =
    resource::list_all_resources::<Server>(None, None, None)
      .await
      .context(
        "Failed to list Servers for Swarm node identity match",
      )?;

  let status_cache = server_status_cache();
  let mut with_cands = Vec::with_capacity(servers.len());
  for server in servers {
    let status = status_cache.get(&server.id).await;
    let cands = swarm_terminal::server_identity_candidates(
      &server,
      status.as_deref(),
    );
    with_cands.push((server, cands));
  }

  swarm_terminal::match_server_to_node_identities(
    &with_cands,
    node_candidates,
  )
  .ok_or_else(|| {
    swarm_terminal::no_server_match_error(node_candidates)
  })
}
