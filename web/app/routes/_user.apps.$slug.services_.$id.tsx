import { useParams } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import { getAppOptions } from '~/queries/apps';
import { getServiceOptions, getServiceStatsOptions } from '~/queries/services';
import { HStack, VStack } from '~/components/interface/stacks';
import { StatusBadge } from '~/components/interface/status-badge';
import { LogStream } from '~/components/global/log-stream';
import { ServiceStatsBar } from '~/components/apps/service-stats-bar';
import { ServiceStatusNotice } from '~/components/apps/service-status-notice';
import { ServiceConnectionCard } from '~/components/apps/service-connection-card';
import { ServiceActionsMenu } from '~/components/apps/service-actions-menu';
import {
  ServiceKindBadge,
  ServiceKindIcon,
} from '~/components/apps/service-kind-badge';
import { describeResources } from '~/components/apps/service-resources';
import { formatRelativeTime } from '~/utils/date';
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

  const { data: serviceData } = useQuery({
    ...getServiceOptions(slug!, id!),
    refetchInterval: (query) => {
      const status = query.state.data?.service.status;
      return status === 'Provisioning' ? 2000 : 5000;
    },
  });
  const { data: stats } = useQuery({
    ...getServiceStatsOptions(slug!, id!),
    refetchInterval: 5000,
  });

  const service = serviceData?.service;
  if (!service) {
    return null;
  }

  const isRunning = service.status === 'Running';
  const meta = [
    ...describeResources(service.resources),
    `Created ${formatRelativeTime(service.created_at)}`,
  ];

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-bg">
      <HStack
        justifyContent="between"
        alignItems="start"
        className="shrink-0 gap-4 border-b border-border px-8 py-5"
      >
        <HStack space={3} alignItems="start" className="min-w-0">
          {/* Optically anchored to the name, not the two-line block. */}
          <ServiceKindIcon kind={service.kind} className="-mt-1" />
          <VStack space={1.5} className="min-w-0">
            <HStack space={3} className="min-w-0">
              <h1 className="truncate font-mono text-[15px] font-semibold text-text">
                {service.name}
              </h1>
              <ServiceKindBadge service={service} />
              {isRunning ? <StatusBadge status={service.status} /> : null}
            </HStack>
            <span className="text-[11px] text-text-tertiary">
              {meta.join(' · ')}
            </span>
          </VStack>
        </HStack>

        <ServiceActionsMenu appSlug={slug!} service={service} />
      </HStack>

      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto px-8 py-6">
        <VStack space={4} className="shrink-0">
          <ServiceStatusNotice appSlug={slug!} service={service} />
          {isRunning ? (
            <ServiceStatsBar service={service} stats={stats} />
          ) : null}
          <ServiceConnectionCard appSlug={slug!} service={service} />
        </VStack>

        <LogStream
          url={`/api/apps/${slug}/services/${id}`}
          title="Logs"
          resourceKind="service"
          className="mt-2 min-h-[320px] flex-1"
        />
      </div>
    </div>
  );
}
