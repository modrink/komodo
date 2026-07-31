import { Types } from "komodo_client";
import DockerResourceLink from "@/components/docker/link";
import { Group } from "@mantine/core";
import ResourceLink from "@/resources/link";
import StackServiceLink from "@/components/stack-service-link";
import SwarmResourceLink from "@/components/swarm/link";

export default function TerminalTargetLink({ target }: { target: Types.TerminalTarget }) {
  switch (target.type) {
    case "Server":
      return <ResourceLink type="Server" id={target.params.server!} />;
    case "Container":
      return (
        <DockerResourceLink
          type="Container"
          serverId={target.params.server}
          name={target.params.container}
        />
      );
    case "Stack":
      return (
        <Group wrap="nowrap">
          <ResourceLink type="Stack" id={target.params.stack} />
          {target.params.service && (
            <StackServiceLink
              id={target.params.stack}
              service={target.params.service}
            />
          )}
        </Group>
      );
    case "Deployment":
      return <ResourceLink type="Deployment" id={target.params.deployment} />;
    case "SwarmTask":
      return (
        <Group wrap="nowrap">
          <ResourceLink type="Swarm" id={target.params.swarm} />
          <SwarmResourceLink
            type="Task"
            swarmId={target.params.swarm}
            resourceId={target.params.task}
            name={target.params.task}
          />
        </Group>
      );
  }
}
