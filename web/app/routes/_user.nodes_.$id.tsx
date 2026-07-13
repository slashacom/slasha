import {
  useSearchParams,
  useParams,
  useNavigate,
  redirect,
} from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { ArrowLeft, ChevronRight, Terminal, Server } from 'lucide-react';
import { getAuthMeOptions } from '~/queries/auth';
import { getNodeOptions } from '~/queries/nodes';
import { HStack, VStack } from '~/components/interface/stacks';
import { LogStream } from '~/components/global/log-stream';
import { queryClient } from '~/utils/query-client';
import { NodeStatusBadge } from '~/components/interface/status-badge';

export async function clientLoader(args: { params: { id: string } }) {
  const { params } = args;
  const me = await queryClient.ensureQueryData(getAuthMeOptions());
  if (me.user.role !== 'Admin') {
    throw redirect('/apps');
  }
  await queryClient.ensureQueryData(getNodeOptions(params.id));
  return null;
}

type MetaItemProps = {
  label: string;
  children: React.ReactNode;
};

function MetaItem(props: MetaItemProps) {
  const { label, children } = props;
  return (
    <VStack space={1}>
      <span className="text-[11px] font-medium uppercase tracking-wider text-text-tertiary">
        {label}
      </span>
      <div className="text-[13px] text-text">{children}</div>
    </VStack>
  );
}

export default function NodeDetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();

  const logType =
    searchParams.get('type') === 'teardown' ? 'teardown' : 'setup';

  const { data: nodeData } = useSuspenseQuery({
    ...getNodeOptions(id!),
    refetchInterval: (query) => {
      const node = query.state.data?.node;
      if (node && (node.status === 'SettingUp' || node.status === 'Deleting')) {
        return 2000;
      }
      return 5000;
    },
  });
  const node = nodeData.node;

  const handleToggleLogType = (type: 'setup' | 'teardown') => {
    setSearchParams({ type });
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-bg">
      <HStack
        justifyContent="between"
        alignItems="center"
        className="shrink-0 gap-4 border-b border-border bg-surface/30 px-8 py-3"
      >
        <HStack space={3} alignItems="center">
          <button
            onClick={() => navigate('/nodes')}
            className="group flex size-7 items-center justify-center rounded border border-border bg-surface transition-all hover:bg-white/[0.06]"
          >
            <ArrowLeft className="size-3.5 text-text-tertiary group-hover:text-text" />
          </button>
          <HStack space={2} alignItems="center">
            <span className="text-[13px] font-medium text-text">Nodes</span>
            <ChevronRight className="size-3 text-text-tertiary" />
            <span className="text-[13px] font-mono text-text-secondary">
              {node.name}
            </span>
          </HStack>
        </HStack>
      </HStack>

      <div className="flex min-h-0 flex-1 flex-col gap-6 p-8">
        <div className="grid grid-cols-1 gap-6 rounded-lg border border-border bg-surface/30 p-6 sm:grid-cols-3">
          <MetaItem label="Node Name">
            <span className="font-medium text-text">{node.name}</span>
          </MetaItem>
          <MetaItem label="Host Connection">
            <span className="font-mono text-text-secondary">
              {node.host
                ? `${node.user}@${node.host}:${node.port}`
                : 'Local Node'}
            </span>
          </MetaItem>
          <MetaItem label="Status">
            <div className="flex items-center">
              <NodeStatusBadge
                status={node.status as any}
                liveStatus={node.live_status}
              />
            </div>
          </MetaItem>
        </div>

        <VStack space={3} className="flex min-h-0 flex-1 flex-col">
          <HStack justifyContent="between" alignItems="center">
            <HStack space={2} alignItems="center">
              <Terminal className="size-4 text-text-tertiary" />
              <h3 className="text-sm font-semibold text-text">Logs</h3>
            </HStack>

            <HStack
              space={1}
              className="rounded border border-border bg-surface p-0.5"
            >
              <button
                type="button"
                onClick={() => handleToggleLogType('setup')}
                className={`h-7 px-3 rounded text-[11px] font-medium transition-colors ${
                  logType === 'setup'
                    ? 'bg-white/[0.08] text-text'
                    : 'text-text-tertiary hover:text-text'
                }`}
              >
                Setup logs
              </button>
              <button
                type="button"
                onClick={() => handleToggleLogType('teardown')}
                className={`h-7 px-3 rounded text-[11px] font-medium transition-colors ${
                  logType === 'teardown'
                    ? 'bg-white/[0.08] text-text'
                    : 'text-text-tertiary hover:text-text'
                }`}
              >
                Teardown logs
              </button>
            </HStack>
          </HStack>

          <LogStream
            url={`/api/nodes/${id}/logs?type=${logType}`}
            emptyMessage={
              logType === 'setup'
                ? 'No setup logs found.'
                : 'No teardown logs found.'
            }
            className="min-h-0 flex-1 rounded-lg border border-border"
          />
        </VStack>
      </div>
    </div>
  );
}
