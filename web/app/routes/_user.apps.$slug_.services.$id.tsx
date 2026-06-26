import { useState } from 'react';
import { useParams, useNavigate } from 'react-router';
import { useQuery, useSuspenseQuery, useQueryClient } from '@tanstack/react-query';
import {
  ArrowLeft,
  ChevronRight,
  ChevronUp,
  Clock,
  Cpu,
  HardDrive,
  MemoryStick,
  MoreHorizontal,
  Pencil,
  Plug,
  RefreshCw,
  RotateCcw,
  Square,
  Terminal,
  Trash2,
} from 'lucide-react';
import { toast } from 'sonner';
import { getAppOptions } from '~/queries/apps';
import {
  getServiceOptions,
  getServiceStatsOptions,
  getServiceEnvVarsOptions,
  useRestartService,
  useRedeployService,
  useStopService,
  useDeleteService,
} from '~/queries/services';
import { HStack, VStack } from '~/components/interface/stacks';
import { StatusBadge } from '~/components/interface/status-badge';
import { LogStream } from '~/components/apps/log-stream';
import { ServiceEnvEditor } from '~/components/apps/service-env-editor';
import { ConnectModal } from '~/components/apps/connect-modal';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '~/components/interface/dropdown-menu';
import { formatFileSize, formatRelativeTime, parseUTC } from '~/utils/format';
import { cn } from '~/utils/classname';
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

function formatUptime(startedAt: string): string {
  const ms = Date.now() - parseUTC(startedAt).getTime();
  if (Number.isNaN(ms) || ms < 0) {
    return '—';
  }
  const seconds = Math.floor(ms / 1000);
  if (seconds < 60) {
    return `${seconds}s`;
  }
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return `${minutes}m`;
  }
  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours}h ${minutes % 60}m`;
  }
  const days = Math.floor(hours / 24);
  return `${days}d ${hours % 24}h`;
}

type StatTileProps = {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  value: string;
  fraction?: number | null;
};

function StatTile(props: StatTileProps) {
  const { icon: Icon, label, value, fraction = null } = props;
  return (
    <div className="rounded-xl border border-border bg-surface/50 px-4 py-4 transition-colors hover:border-white/15">
      <HStack space={1.5} alignItems="center" className="text-text-tertiary">
        <Icon className="size-3.5" />
        <span className="text-[10px] font-medium uppercase tracking-wider">
          {label}
        </span>
      </HStack>
      <div className="mt-2 font-mono text-[22px] font-medium leading-none tracking-tight text-text">
        {value}
      </div>
      {fraction != null ? (
        <HStack space={2} alignItems="center" className="mt-2.5">
          <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-white/10">
            <div
              className={cn(
                'h-full rounded-full',
                fraction >= 0.95
                  ? 'bg-red-500'
                  : fraction >= 0.8
                    ? 'bg-amber-400'
                    : 'bg-emerald-400/80'
              )}
              style={{ width: `${Math.min(100, Math.max(2, fraction * 100))}%` }}
            />
          </div>
          <span className="text-[10px] tabular-nums text-text-tertiary">
            {Math.round(fraction * 100)}%
          </span>
        </HStack>
      ) : null}
    </div>
  );
}

export default function ServiceDetailPage() {
  const { slug, id } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [showConnect, setShowConnect] = useState(false);
  const [showRestartConfirm, setShowRestartConfirm] = useState(false);
  const [showRedeployConfirm, setShowRedeployConfirm] = useState(false);
  const [showStopConfirm, setShowStopConfirm] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [configOpen, setConfigOpen] = useState(false);

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
  const { data: envData } = useQuery(getServiceEnvVarsOptions(slug!, id!));

  const restartService = useRestartService();
  const redeployService = useRedeployService();
  const stopService = useStopService();
  const deleteService = useDeleteService();

  const app = appData.app;
  const service = serviceData?.service;
  if (!service) {
    return null;
  }

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ['apps', slug, 'services'] });
  };

  const isRunning = service.status === 'Running';
  const canRestart = isRunning || service.status === 'Stopped';
  const canRedeploy =
    isRunning || service.status === 'Stopped' || service.status === 'Failed';

  const handleRestart = async () => {
    try {
      await restartService.mutateAsync({ appSlug: slug!, serviceId: id! });
      toast.success('Service restart triggered.');
      invalidate();
      setShowRestartConfirm(false);
    } catch (err) {
      toast.error('Failed to restart service: ' + err);
    }
  };

  const handleRedeploy = async () => {
    try {
      await redeployService.mutateAsync({ appSlug: slug!, serviceId: id! });
      toast.success('Service redeploy started.');
      invalidate();
      setShowRedeployConfirm(false);
    } catch (err) {
      toast.error('Failed to redeploy service: ' + err);
    }
  };

  const handleStop = async () => {
    try {
      await stopService.mutateAsync({ appSlug: slug!, serviceId: id! });
      invalidate();
      setShowStopConfirm(false);
    } catch (err) {
      toast.error('Failed to stop service: ' + err);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteService.mutateAsync({ appSlug: slug!, serviceId: id! });
      invalidate();
      navigate(`/apps/${slug}/services`);
    } catch (err) {
      toast.error('Failed to delete service: ' + err);
    }
  };

  const envKeys = Object.keys(envData?.env_vars ?? {}).sort();
  const exampleKey = envKeys.includes('DATABASE_URL')
    ? 'DATABASE_URL'
    : (envKeys[0] ?? 'DATABASE_URL');

  // Only frame memory as used/limit when the service has an explicit cap;
  // otherwise the "limit" is the whole host's RAM, which is misleading.
  const memUsed = stats?.memory_used_bytes ?? null;
  const memLimit =
    service.resources?.memory_bytes != null
      ? Number(service.resources.memory_bytes)
      : null;
  const memValue =
    memUsed == null
      ? '—'
      : memLimit != null
        ? `${formatFileSize(memUsed)} / ${formatFileSize(memLimit)}`
        : formatFileSize(memUsed);
  const memFraction =
    memUsed != null && memLimit != null && memLimit > 0
      ? memUsed / memLimit
      : null;

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
            <span className="rounded bg-white/5 px-1.5 py-0.5 text-[11px] font-medium text-text-secondary">
              {service.kind} {service.version}
            </span>
          </HStack>
        </HStack>

        <HStack space={3} alignItems="center">
          <StatusBadge status={service.status} />
          <span className="text-[11px] text-text-tertiary">
            Created {formatRelativeTime(service.created_at)}
          </span>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                aria-label="Service actions"
                className="flex size-7 items-center justify-center rounded-md border border-border bg-surface text-text-tertiary transition-all hover:bg-white/[0.06] hover:text-text data-[state=open]:bg-white/[0.06] data-[state=open]:text-text"
              >
                <MoreHorizontal className="size-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {isRunning ? (
                <DropdownMenuItem onClick={() => setShowConnect(true)}>
                  <Plug className="size-3.5" />
                  Connect
                </DropdownMenuItem>
              ) : null}
              {canRestart ? (
                <DropdownMenuItem onClick={() => setShowRestartConfirm(true)}>
                  <RefreshCw className="size-3.5" />
                  Restart
                </DropdownMenuItem>
              ) : null}
              {canRedeploy ? (
                <DropdownMenuItem onClick={() => setShowRedeployConfirm(true)}>
                  <RotateCcw className="size-3.5" />
                  Redeploy
                </DropdownMenuItem>
              ) : null}
              {isRunning ? (
                <DropdownMenuItem onClick={() => setShowStopConfirm(true)}>
                  <Square className="size-3.5" />
                  Stop
                </DropdownMenuItem>
              ) : null}
              <DropdownMenuSeparator />
              <DropdownMenuItem
                variant="destructive"
                onClick={() => setShowDeleteConfirm(true)}
              >
                <Trash2 className="size-3.5" />
                Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </HStack>
      </HStack>

      <div className="flex min-h-0 flex-1 flex-col gap-6 overflow-auto p-8">
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <StatTile
            icon={Clock}
            label="Uptime"
            value={
              isRunning && stats?.started_at
                ? formatUptime(stats.started_at)
                : '—'
            }
          />
          <StatTile
            icon={Cpu}
            label="CPU"
            value={
              stats?.cpu_percent != null
                ? `${stats.cpu_percent.toFixed(1)}%`
                : '—'
            }
          />
          <StatTile
            icon={MemoryStick}
            label="Memory"
            value={memValue}
            fraction={memFraction}
          />
          <StatTile
            icon={HardDrive}
            label="Disk"
            value={
              stats?.disk_bytes != null ? formatFileSize(stats.disk_bytes) : '—'
            }
          />
        </div>

        {configOpen ? (
          <div className="relative">
            <button
              type="button"
              onClick={() => setConfigOpen(false)}
              aria-label="Collapse configuration"
              className="absolute right-5 top-5 z-10 flex size-7 cursor-pointer items-center justify-center rounded-md text-text-tertiary transition-colors hover:bg-white/5 hover:text-text"
            >
              <ChevronUp className="size-4" />
            </button>
            <ServiceEnvEditor
              appSlug={slug!}
              serviceId={id!}
              serviceName={service.name}
            />
          </div>
        ) : (
          <button
            type="button"
            onClick={() => setConfigOpen(true)}
            className="group block w-full cursor-pointer rounded-xl border border-border bg-surface/50 p-4 text-left transition-colors hover:border-white/15 hover:bg-surface/70"
          >
            <HStack justifyContent="between" alignItems="start" space={4}>
              <VStack space={2} className="min-w-0">
                <span className="text-sm font-semibold text-text">
                  Configuration
                </span>
                <HStack space={1.5} wrap>
                  {envKeys.map((key) => (
                    <span
                      key={key}
                      className="rounded bg-white/5 px-1.5 py-0.5 font-mono text-[11px] text-text-secondary"
                    >
                      {key}
                    </span>
                  ))}
                </HStack>
                <p className="text-[11px] leading-5 text-text-tertiary">
                  Reference these from your app as{' '}
                  <span className="font-mono text-text-secondary">
                    {`\${{ ${service.name}.${exampleKey} }}`}
                  </span>
                  .
                </p>
              </VStack>
              <span className="flex size-7 shrink-0 items-center justify-center rounded-md text-text-tertiary transition-colors group-hover:bg-white/5 group-hover:text-text">
                <Pencil className="size-4" />
              </span>
            </HStack>
          </button>
        )}

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

      {showConnect ? (
        <ConnectModal
          appSlug={slug!}
          service={service}
          onClose={() => setShowConnect(false)}
        />
      ) : null}

      <ConfirmationDialog
        open={showRestartConfirm}
        onOpenChange={setShowRestartConfirm}
        title="Restart Service"
        description={`Restart ${service.name}? It will be briefly unavailable while it restarts.`}
        confirmLabel="Restart"
        onConfirm={handleRestart}
      />

      <ConfirmationDialog
        open={showRedeployConfirm}
        onOpenChange={setShowRedeployConfirm}
        title="Redeploy Service"
        description={`Redeploy ${service.name}? The container is recreated, so it will be briefly unavailable.`}
        confirmLabel="Redeploy"
        onConfirm={handleRedeploy}
      />

      <ConfirmationDialog
        open={showStopConfirm}
        onOpenChange={setShowStopConfirm}
        title="Stop Service"
        description={`Stop ${service.name}? Apps using it will lose their connection until it is restarted.`}
        confirmLabel="Stop"
        onConfirm={handleStop}
      />

      <ConfirmationDialog
        open={showDeleteConfirm}
        onOpenChange={setShowDeleteConfirm}
        title="Delete Service"
        description={`Are you sure you want to delete ${service.name}? All underlying data will be permanently destroyed.`}
        confirmLabel="Delete Service"
        onConfirm={handleDelete}
      />
    </div>
  );
}
