import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  AlertCircle,
  CheckCircle2,
  XCircle,
  CircleDashed,
  Terminal,
  Trash2,
  Settings,
  Plug,
  Square,
  RotateCcw,
  RefreshCw,
} from 'lucide-react';
import type { Service, ServiceStatus } from '~/models/service';
import {
  useStopService,
  useDeleteService,
  useRestartService,
  useRedeployService,
} from '~/queries/services';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import { formatRelativeTime } from '~/utils/format';
import { toast } from 'sonner';
import {
  ConnectModal,
  ServiceConfigModal,
} from '~/components/apps/service-modals';

function StatusBadge(props: { status: ServiceStatus }) {
  const { status } = props;
  const configs: Record<
    ServiceStatus,
    { icon: any; color: string; bg: string }
  > = {
    Provisioning: {
      icon: CircleDashed,
      color: 'text-sky-400',
      bg: 'bg-sky-400/10',
    },
    Running: {
      icon: CheckCircle2,
      color: 'text-emerald-400',
      bg: 'bg-emerald-400/10',
    },
    Failed: { icon: XCircle, color: 'text-red-400', bg: 'bg-red-400/10' },
    Stopped: {
      icon: AlertCircle,
      color: 'text-text-tertiary',
      bg: 'bg-white/5',
    },
  };

  const { icon: Icon, color, bg } = configs[status];

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded px-2 py-0.5 text-[11px] font-medium',
        color,
        bg
      )}
    >
      <Icon
        className={cn('size-3', status === 'Provisioning' && 'animate-spin')}
      />
      {status}
    </span>
  );
}

export function ServiceRow(props: {
  service: Service;
  appSlug: string;
  onShowLogs: () => void;
}) {
  const { service, appSlug, onShowLogs } = props;
  const queryClient = useQueryClient();
  const stopService = useStopService();
  const deleteService = useDeleteService();
  const restartService = useRestartService();
  const redeployService = useRedeployService();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [showConfig, setShowConfig] = useState(false);
  const [showConnectModal, setShowConnectModal] = useState(false);

  const handleStop = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await stopService.mutateAsync({
        appSlug,
        serviceId: service.id,
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services'],
      });
    } catch (err) {
      toast.error('Failed to stop service: ' + err);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteService.mutateAsync({
        appSlug,
        serviceId: service.id,
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services'],
      });
      setShowDeleteConfirm(false);
    } catch (e) {
      toast.error('Failed to delete service: ' + e);
    }
  };

  const handleRestart = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await restartService.mutateAsync({
        appSlug,
        serviceId: service.id,
      });
      toast.success('Service restart triggered.');
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services'],
      });
    } catch (err) {
      toast.error('Failed to restart service: ' + err);
    }
  };

  const handleRedeploy = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await redeployService.mutateAsync({
        appSlug,
        serviceId: service.id,
      });
      toast.success('Service redeploy started.');
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services'],
      });
    } catch (err) {
      toast.error('Failed to redeploy service: ' + err);
    }
  };

  return (
    <>
      <div className="group grid grid-cols-[1fr_auto] items-center gap-4 px-8 py-4 transition-colors hover:bg-white/[0.02]">
        <VStack space={1.5}>
          <HStack space={3}>
            <span className="font-mono text-[13px] font-semibold text-text">
              {service.name}
            </span>
            <span className="text-[11px] font-medium text-text-secondary bg-surface-hover px-1.5 py-0.5 rounded">
              {service.kind} {service.version}
            </span>
            <StatusBadge status={service.status} />
          </HStack>
          <HStack space={3}>
            <span className="text-[11px] text-text-tertiary">
              slasha-svc-{service.id.slice(0, 8)}
            </span>
            <span className="text-[11px] text-text-tertiary">
              Created {formatRelativeTime(service.created_at)}
            </span>
          </HStack>
        </VStack>

        <HStack space={2}>
          <Button
            label="Logs"
            icon={<Terminal className="size-3.5" />}
            variant="ghost"
            size="sm"
            color="neutral"
            onClick={onShowLogs}
          />
          {service.status === 'Running' && (
            <Button
              label="Connect"
              icon={<Plug className="size-3.5" />}
              variant="ghost"
              size="sm"
              color="neutral"
              onClick={(e) => {
                e.stopPropagation();
                setShowConnectModal(true);
              }}
            />
          )}
          {(service.status === 'Running' || service.status === 'Stopped') && (
            <Button
              label="Restart"
              icon={<RefreshCw className="size-3.5" />}
              variant="ghost"
              size="sm"
              color="neutral"
              onClick={handleRestart}
              isLoading={restartService.isPending}
            />
          )}
          {(service.status === 'Running' ||
            service.status === 'Stopped' ||
            service.status === 'Failed') && (
            <Button
              label="Redeploy"
              icon={<RotateCcw className="size-3.5" />}
              variant="ghost"
              size="sm"
              color="neutral"
              onClick={handleRedeploy}
              isLoading={redeployService.isPending}
            />
          )}
          {service.status === 'Running' && (
            <Button
              label="Stop"
              icon={<Square className="size-3.5" />}
              variant="ghost"
              size="sm"
              color="error"
              onClick={handleStop}
              isLoading={stopService.isPending}
            />
          )}
          <Button
            label="Settings"
            icon={<Settings className="size-3.5" />}
            variant="ghost"
            size="sm"
            onClick={(e) => {
              e.stopPropagation();
              setShowConfig(true);
            }}
          />
          <Button
            label="Delete"
            icon={<Trash2 className="size-3.5" />}
            variant="ghost"
            size="sm"
            color="error"
            onClick={(e) => {
              e.stopPropagation();
              setShowDeleteConfirm(true);
            }}
            isLoading={deleteService.isPending}
          />
        </HStack>
      </div>

      <ConfirmationDialog
        open={showDeleteConfirm}
        onOpenChange={setShowDeleteConfirm}
        title="Delete Service"
        description={`Are you sure you want to delete ${service.name}? All underlying data will be permanently destroyed.`}
        confirmLabel="Delete Service"
        onConfirm={handleDelete}
      />

      {showConfig && (
        <ServiceConfigModal
          appSlug={appSlug}
          service={service}
          onClose={() => setShowConfig(false)}
        />
      )}

      {showConnectModal && (
        <ConnectModal
          appSlug={appSlug}
          service={service}
          onClose={() => setShowConnectModal(false)}
        />
      )}
    </>
  );
}
