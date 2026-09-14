import { useParams, useSearchParams } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import { getAppOptions } from '~/queries/apps';
import {
  getServiceOptions,
  getServiceStatsOptions,
  isServiceTransitional,
} from '~/queries/services';
import { HStack, VStack } from '~/components/interface/stacks';
import { StatusBadge } from '~/components/interface/status-badge';
import { LogStream } from '~/components/global/log-stream';
import { ServiceStatsBar } from '~/components/apps/service-stats-bar';
import { ServiceStatusNotice } from '~/components/apps/service-status-notice';
import { ServiceConnectionCard } from '~/components/apps/service-connection-card';
import { ServiceActionsMenu } from '~/components/apps/service-actions-menu';
import { ServiceKindBadge } from '~/components/apps/service-kind-badge';
import { describeResources } from '~/components/apps/service-resources';
import { ServiceBackupManager } from '~/components/apps/service-backup-manager';
import { formatRelativeTime } from '~/utils/date';
import { queryClient } from '~/utils/query-client';
import { cn } from '~/utils/classname';
import {
  getServiceBackupConfigOptions,
  getServiceBackupsOptions,
} from '~/queries/service-backups';
import { getS3StoragesOptions } from '~/queries/s3-storage';

export async function clientLoader(args: {
  params: { slug: string; id: string };
}) {
  const { params } = args;
  const [_, serviceData] = await Promise.all([
    queryClient.query({ ...getAppOptions(params.slug), staleTime: 'static' }),
    queryClient.query({
      ...getServiceOptions(params.slug, params.id),
      staleTime: 'static',
    }),
  ]);

  if (serviceData?.service.kind !== 'Redis') {
    await Promise.all([
      queryClient.query({
        ...getServiceBackupConfigOptions(params.slug, params.id),
        staleTime: 'static',
      }),
      queryClient.query({
        ...getServiceBackupsOptions(params.slug, params.id),
        staleTime: 'static',
      }),
      queryClient.query({
        ...getS3StoragesOptions(),
        staleTime: 'static',
      }),
    ]);
  }
}

const allTabs = [
  { id: 'overview', label: 'Overview' },
  { id: 'logs', label: 'Logs' },
  { id: 'backups', label: 'Backups' },
] as const;

type ServiceTab = (typeof allTabs)[number]['id'];

export default function ServiceDetailPage() {
  const { slug, id } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const currentTab = (searchParams.get('tab') || 'overview') as ServiceTab;

  const { data: serviceData } = useQuery({
    ...getServiceOptions(slug!, id!),
    refetchInterval: (query) => {
      const status =
        query.state.data?.runtime_status || query.state.data?.service.status;
      return status && isServiceTransitional(status) ? 2000 : 5000;
    },
  });
  const { data: stats } = useQuery({
    ...getServiceStatsOptions(slug!, id!),
    refetchInterval: 5000,
  });

  const service = serviceData?.service;
  const runtimeStatus = serviceData?.runtime_status;

  if (!service) {
    return null;
  }

  const supportsBackups = service.kind !== 'Redis';
  const tabs = allTabs.filter((tab) => tab.id !== 'backups' || supportsBackups);

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
          <VStack space={1.5} className="min-w-0">
            <HStack space={3} className="min-w-0">
              <h1 className="truncate font-mono text-[15px] font-semibold text-text">
                {service.name}
              </h1>
              <ServiceKindBadge service={service} />
              <StatusBadge status={runtimeStatus || service.status} />
            </HStack>
            <span className="text-[11px] text-text-tertiary">
              {meta.join(' · ')}
            </span>
          </VStack>
        </HStack>

        <ServiceActionsMenu appSlug={slug!} service={service} />
      </HStack>

      <div className="shrink-0 border-b border-border bg-surface/30 px-8">
        <nav className="-mb-px flex gap-6" aria-label="Service Tabs">
          {tabs.map((tab) => {
            const isActive = currentTab === tab.id;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => {
                  setSearchParams(
                    (prev) => {
                      if (tab.id === 'overview') {
                        prev.delete('tab');
                      } else {
                        prev.set('tab', tab.id);
                      }
                      return prev;
                    },
                    { replace: true }
                  );
                }}
                className={cn(
                  'flex h-10 cursor-pointer items-center whitespace-nowrap border-b-2 text-[13px] font-medium transition-colors',
                  isActive
                    ? 'border-white text-text'
                    : 'border-transparent text-text-tertiary hover:text-text-secondary'
                )}
              >
                {tab.label}
              </button>
            );
          })}
        </nav>
      </div>

      {currentTab === 'overview' && (
        <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto px-8 py-6">
          <VStack space={4} className="shrink-0">
            <ServiceStatusNotice
              appSlug={slug!}
              service={service}
              runtimeStatus={runtimeStatus}
            />
            {isRunning ? (
              <ServiceStatsBar service={service} stats={stats} />
            ) : null}
            <ServiceConnectionCard appSlug={slug!} service={service} />
          </VStack>
        </div>
      )}

      {currentTab === 'logs' && (
        <div className="flex min-h-0 flex-1 flex-col px-8 py-6">
          <LogStream
            url={`/api/apps/${slug}/services/${id}`}
            title="Logs"
            resourceKind="service"
            className="min-h-0 flex-1"
          />
        </div>
      )}

      {currentTab === 'backups' && (
        <div className="flex min-h-0 flex-1 flex-col overflow-auto px-8 py-6">
          <ServiceBackupManager
            appSlug={slug!}
            service={service}
            runtimeStatus={runtimeStatus}
          />
        </div>
      )}
    </div>
  );
}
