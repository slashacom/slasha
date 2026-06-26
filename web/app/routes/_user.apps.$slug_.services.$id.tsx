import { useParams, useNavigate } from 'react-router';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { ArrowLeft, ChevronRight, Terminal } from 'lucide-react';
import { getAppOptions } from '~/queries/apps';
import { getServiceOptions, getServiceStatsOptions } from '~/queries/services';
import { HStack, VStack } from '~/components/interface/stacks';
import { StatusBadge } from '~/components/interface/status-badge';
import { LogStream } from '~/components/apps/log-stream';
import { ServiceStatsBar } from '~/components/apps/service-stats-bar';
import { ServiceConfigCard } from '~/components/apps/service-config-card';
import { ServiceActionsMenu } from '~/components/apps/service-actions-menu';
import { ServiceKindBadge } from '~/components/apps/service-kind-badge';
import { formatRelativeTime } from '~/utils/format';
import { queryClient } from '~/utils/query-client';

export async function clientLoader(args: {
  params: { slug: string; id: string };
}) {
  const { params } = args;
  await Promise.all([
    queryClient.ensureQueryData(getAppOptions(params.slug)),
    queryClient.ensureQueryData(getServiceOptions(params.slug, params.id)),
  ]);
}

export default function ServiceDetailPage() {
  const { slug, id } = useParams();
  const navigate = useNavigate();

  const { data: appData } = useSuspenseQuery(getAppOptions(slug!));
  const { data: serviceData } = useQuery({
    ...getServiceOptions(slug!, id!),
    refetchInterval: (query) =>
      query.state.data?.service.status === 'Provisioning' ? 2000 : false,
  });
  const { data: stats } = useQuery({
    ...getServiceStatsOptions(slug!, id!),
    refetchInterval: 5000,
  });

  const app = appData.app;
  const service = serviceData?.service;
  if (!service) {
    return null;
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-bg">
      <HStack
        justifyContent="between"
        alignItems="center"
        className="shrink-0 gap-4 border-b border-border bg-surface/30 px-8 py-3"
      >
        <HStack space={3} alignItems="center">
          <button
            onClick={() => navigate(`/apps/${slug}/services`)}
            className="group flex size-7 items-center justify-center rounded border border-border bg-surface transition-all hover:bg-white/[0.06]"
          >
            <ArrowLeft className="size-3.5 text-text-tertiary group-hover:text-text" />
          </button>
          <HStack space={2} alignItems="center">
            <span className="text-[13px] font-medium text-text">
              {app.name}
            </span>
            <ChevronRight className="size-3 text-text-tertiary" />
            <span className="font-mono text-[13px] text-text">
              {service.name}
            </span>
            <ServiceKindBadge service={service} />
          </HStack>
        </HStack>

        <HStack space={3} alignItems="center">
          <StatusBadge status={service.status} />
          <span className="text-[11px] text-text-tertiary">
            Created {formatRelativeTime(service.created_at)}
          </span>
          <ServiceActionsMenu appSlug={slug!} service={service} />
        </HStack>
      </HStack>

      <div className="flex min-h-0 flex-1 flex-col gap-6 overflow-auto p-8">
        <ServiceStatsBar service={service} stats={stats} />
        <ServiceConfigCard appSlug={slug!} service={service} />

        <VStack space={3} className="flex min-h-0 flex-1 flex-col">
          <HStack space={2} alignItems="center">
            <Terminal className="size-4 text-text-tertiary" />
            <h3 className="text-sm font-semibold text-text">Logs</h3>
          </HStack>
          <LogStream
            url={`/api/apps/${slug}/services/${id}/logs`}
            className="min-h-[24rem] flex-1 rounded-lg border border-border"
          />
        </VStack>
      </div>
    </div>
  );
}
