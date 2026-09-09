import { useNavigate } from 'react-router';
import { HardDrive, Network } from 'lucide-react';
import type { NodeWithInfo } from '~/queries/nodes';
import { NodeStatusBadge } from '~/components/interface/status-badge';
import { HStack, VStack } from '~/components/interface/stacks';
import { formatRelativeTime } from '~/utils/date';

type NodeCardProps = {
  node: NodeWithInfo;
};

export function NodeCard(props: NodeCardProps) {
  const { node } = props;
  const navigate = useNavigate();
  const isLocal = node.id === 'local';

  return (
    <div
      role="link"
      tabIndex={0}
      onClick={() => navigate(`/nodes/${node.id}`)}
      onKeyDown={(event) => {
        if (event.key !== 'Enter') {
          return;
        }

        navigate(`/nodes/${node.id}`);
      }}
      className="group cursor-pointer rounded-lg border border-border bg-surface/60 p-4 transition-colors hover:bg-surface focus-visible:bg-surface focus-visible:outline-none"
    >
      <HStack justifyContent="between" alignItems="start" className="gap-2">
        <VStack space={0.5} className="min-w-0">
          <span className="truncate text-[14px] font-medium text-text">
            {node.name}
          </span>
          <code className="truncate font-mono text-[12px] text-text-tertiary">
            {isLocal ? 'local' : `${node.user}@${node.host}:${node.port}`}
          </code>
        </VStack>
        <NodeStatusBadge
          status={node.status}
          connectionStatus={node.connection_status}
        />
      </HStack>

      <HStack
        justifyContent="between"
        className="mt-4 gap-2 border-t border-border/60 pt-3"
      >
        <HStack space={1.5} className="min-w-0">
          {isLocal ? (
            <HardDrive className="size-3 shrink-0 text-text-tertiary" />
          ) : (
            <Network className="size-3 shrink-0 text-text-tertiary" />
          )}
          <span className="truncate text-[11px] capitalize text-text-tertiary">
            {node.os || 'Unknown OS'}
          </span>
        </HStack>
        <span className="shrink-0 whitespace-nowrap text-[11px] text-text-tertiary/70">
          Added {formatRelativeTime(node.created_at)}
        </span>
      </HStack>
    </div>
  );
}
