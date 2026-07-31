//! Swarm task terminal resolve helpers (node ↔ Server identity matching).

use anyhow::anyhow;
use komodo_client::entities::{
  docker::{
    node::{SwarmNode, SwarmNodeListItem},
    task::TaskState,
  },
  server::Server,
  stats::SystemInformation,
};

use crate::state::CachedServerStatus;

/// Validate task is runnable for create/connect when `require_running`.
pub fn check_swarm_task_running_gate(
  task_id: &str,
  state: Option<TaskState>,
  container_id: Option<&str>,
  require_running: bool,
) -> anyhow::Result<()> {
  if !require_running {
    return Ok(());
  }
  if state != Some(TaskState::RUNNING) {
    return Err(anyhow!(
      "Swarm task {task_id} is not running (state: {state:?})"
    ));
  }
  if container_id.is_none_or(|id| id.is_empty()) {
    return Err(anyhow!(
      "Swarm task {task_id} has no container ID"
    ));
  }
  Ok(())
}

/// Strip host:port → host; also accept raw IPs / hostnames.
pub fn normalize_host_candidate(raw: &str) -> Option<String> {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return None;
  }
  // URL form: http://host:port/... or https://...
  if let Ok(url) = url::Url::parse(trimmed) {
    if let Some(host) = url.host_str() {
      let host = host.trim();
      if !host.is_empty() {
        return Some(host.to_string());
      }
    }
  }
  // host:port (not a URL)
  if let Some((host, port)) = trimmed.rsplit_once(':')
    && !host.is_empty()
    && !host.contains(']') // skip IPv6 for simple split
    && port.chars().all(|c| c.is_ascii_digit())
  {
    return Some(host.to_string());
  }
  // [ipv6]:port
  if trimmed.starts_with('[')
    && let Some(end) = trimmed.find(']')
  {
    let host = &trimmed[1..end];
    if !host.is_empty() {
      return Some(host.to_string());
    }
  }
  Some(trimmed.to_string())
}

fn short_hostname(host: &str) -> &str {
  host.split('.').next().unwrap_or(host)
}

/// Exact ignore-case, then short-name (before first `.`) either direction.
pub fn identities_match(a: &str, b: &str) -> bool {
  let a = a.trim();
  let b = b.trim();
  if a.is_empty() || b.is_empty() {
    return false;
  }
  if a.eq_ignore_ascii_case(b) {
    return true;
  }
  let a_short = short_hostname(a);
  let b_short = short_hostname(b);
  a_short.eq_ignore_ascii_case(b)
    || b_short.eq_ignore_ascii_case(a)
    || a_short.eq_ignore_ascii_case(b_short)
}

pub fn push_unique_candidate(out: &mut Vec<String>, raw: Option<&str>) {
  let Some(raw) = raw else {
    return;
  };
  let Some(norm) = normalize_host_candidate(raw) else {
    return;
  };
  if out
    .iter()
    .any(|existing| existing.eq_ignore_ascii_case(&norm))
  {
    return;
  }
  out.push(norm);
}

pub fn node_identity_candidates_from_list_item(
  node: &SwarmNodeListItem,
) -> Vec<String> {
  let mut out = Vec::new();
  push_unique_candidate(&mut out, node.hostname.as_deref());
  push_unique_candidate(&mut out, node.name.as_deref());
  push_unique_candidate(&mut out, node.manager_addr.as_deref());
  out
}

pub fn node_identity_candidates_from_inspect(
  node: &SwarmNode,
) -> Vec<String> {
  let mut out = Vec::new();
  push_unique_candidate(
    &mut out,
    node.description.as_ref().and_then(|d| d.hostname.as_deref()),
  );
  push_unique_candidate(
    &mut out,
    node.spec.as_ref().and_then(|s| s.name.as_deref()),
  );
  push_unique_candidate(
    &mut out,
    node.status.as_ref().and_then(|s| s.addr.as_deref()),
  );
  push_unique_candidate(
    &mut out,
    node
      .manager_status
      .as_ref()
      .and_then(|s| s.addr.as_deref()),
  );
  out
}

pub fn server_identity_candidates(
  server: &Server,
  status: Option<&CachedServerStatus>,
) -> Vec<String> {
  let mut out = Vec::new();
  push_unique_candidate(&mut out, Some(server.name.as_str()));
  push_unique_candidate(&mut out, Some(server.config.address.as_str()));
  if !server.config.external_address.is_empty() {
    push_unique_candidate(
      &mut out,
      Some(server.config.external_address.as_str()),
    );
  }
  if let Some(status) = status {
    if let Some(SystemInformation {
      host_name: Some(host_name),
      ..
    }) = &status.system_info
    {
      push_unique_candidate(&mut out, Some(host_name.as_str()));
    }
    if let Some(info) = &status.periphery_info
      && let Some(ip) = &info.public_ip
    {
      push_unique_candidate(&mut out, Some(ip.as_str()));
    }
  }
  out
}

/// First Server whose identity candidates match any node candidate.
pub fn match_server_to_node_identities(
  servers: &[(Server, Vec<String>)],
  node_candidates: &[String],
) -> Option<Server> {
  for (server, server_cands) in servers {
    for sc in server_cands {
      for nc in node_candidates {
        if identities_match(sc, nc) {
          return Some(server.clone());
        }
      }
    }
  }
  None
}

pub fn no_server_match_error(node_candidates: &[String]) -> anyhow::Error {
  let listed = if node_candidates.is_empty() {
    "(none)".to_string()
  } else {
    node_candidates.join(", ")
  };
  anyhow!(
    "No Komodo Server matches Swarm node identity candidates [{listed}]. \
     Add the node as a Server with Periphery installed, matching Server name, \
     system hostname, address host, or public IP."
  )
}

pub fn require_node_candidates(
  node_id: &str,
  candidates: Vec<String>,
) -> anyhow::Result<Vec<String>> {
  if candidates.is_empty() {
    return Err(anyhow!(
      "Swarm node {node_id} has no hostname / name / address to match"
    ));
  }
  Ok(candidates)
}

#[cfg(test)]
mod tests {
  use super::*;
  use komodo_client::entities::server::ServerConfig;

  fn server_named(name: &str, address: &str) -> Server {
    Server {
      id: format!("id-{name}"),
      name: name.to_string(),
      config: ServerConfig {
        address: address.to_string(),
        ..Default::default()
      },
      ..Default::default()
    }
  }

  #[test]
  fn normalize_strips_url_and_port() {
    assert_eq!(
      normalize_host_candidate("https://worker-1.example:8120"),
      Some("worker-1.example".into())
    );
    assert_eq!(
      normalize_host_candidate("10.0.0.5:2377"),
      Some("10.0.0.5".into())
    );
    assert_eq!(
      normalize_host_candidate("worker-1"),
      Some("worker-1".into())
    );
  }

  #[test]
  fn identities_exact_and_short_vs_fqdn() {
    assert!(identities_match("Node1", "node1"));
    assert!(identities_match("node1", "node1.cluster.local"));
    assert!(identities_match("node1.cluster.local", "node1"));
    assert!(!identities_match("node1", "node2"));
  }

  #[test]
  fn name_exact_match() {
    let server = server_named("worker-1", "http://127.0.0.1:8120");
    let servers = vec![(
      server.clone(),
      server_identity_candidates(&server, None),
    )];
    let node = vec!["worker-1".to_string()];
    let matched = match_server_to_node_identities(&servers, &node);
    assert_eq!(matched.unwrap().id, server.id);
  }

  #[test]
  fn host_name_exact_via_candidates() {
    let server = server_named("my-server", "http://127.0.0.1:8120");
    let mut cands = server_identity_candidates(&server, None);
    push_unique_candidate(&mut cands, Some("prod-worker.local"));
    let servers = vec![(server.clone(), cands)];
    let node = vec!["prod-worker.local".to_string()];
    assert!(match_server_to_node_identities(&servers, &node).is_some());
  }

  #[test]
  fn short_vs_fqdn_match() {
    let server = server_named("node1", "http://127.0.0.1:8120");
    let servers = vec![(
      server.clone(),
      server_identity_candidates(&server, None),
    )];
    let node = vec!["node1.cluster.local".to_string()];
    assert!(match_server_to_node_identities(&servers, &node).is_some());
  }

  #[test]
  fn address_host_match() {
    let server =
      server_named("s1", "https://worker-a.internal:8120/path");
    let servers = vec![(
      server.clone(),
      server_identity_candidates(&server, None),
    )];
    let node = vec!["worker-a.internal".to_string()];
    assert!(match_server_to_node_identities(&servers, &node).is_some());
  }

  #[test]
  fn no_match_error_lists_candidates() {
    let err = no_server_match_error(&["node-x".into(), "10.1.1.1".into()]);
    let msg = format!("{err:#}");
    assert!(msg.contains("node-x"));
    assert!(msg.contains("10.1.1.1"));
  }

  #[test]
  fn running_gate() {
    check_swarm_task_running_gate(
      "t1",
      Some(TaskState::RUNNING),
      Some("abc"),
      true,
    )
    .unwrap();
    assert!(check_swarm_task_running_gate(
      "t1",
      Some(TaskState::SHUTDOWN),
      Some("abc"),
      true,
    )
    .is_err());
    assert!(check_swarm_task_running_gate(
      "t1",
      Some(TaskState::RUNNING),
      None,
      true,
    )
    .is_err());
    check_swarm_task_running_gate(
      "t1",
      Some(TaskState::SHUTDOWN),
      None,
      false,
    )
    .unwrap();
  }
}
