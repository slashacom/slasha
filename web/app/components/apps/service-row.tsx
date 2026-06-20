import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  MoreHorizontal,
  Plug,
  RefreshCw,
  RotateCcw,
  Settings,
  Square,
  Terminal,
  Trash2,
} from 'lucide-react';
import type { Service } from '~/models/service';
import {
  useStopService,
  useDeleteService,
  useRestartService,
  useRedeployService,
} from '~/queries/services';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '~/components/interface/dropdown-menu';
import { HStack, VStack } from '~/components/interface/stacks';
import { StatusBadge } from '~/components/interface/status-badge';
import { formatRelativeTime } from '~/utils/format';
import { toast } from 'sonner';
import { ConnectModal } from '~/components/apps/connect-modal';
import { ServiceConfigModal } from '~/components/apps/service-config-modal';

type ServiceRowProps = {
  service: Service;
  appSlug: string;
  onShowLogs: () => void;
};

export function ServiceRow(props: ServiceRowProps) {
  const { service, appSlug, onShowLogs } = props;
  const queryClient = useQueryClient();
  const stopService = useStopService();
  const deleteService = useDeleteService();
  const restartService = useRestartService();
  const redeployService = useRedeployService();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [showStopConfirm, setShowStopConfirm] = useState(false);
  const [showConfig, setShowConfig] = useState(false);
  const [showConnectModal, setShowConnectModal] = useState(false);

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'services'] });
  };

  const handleStop = async () => {
    try {
      await stopService.mutateAsync({ appSlug, serviceId: service.id });
      invalidate();
      setShowStopConfirm(false);
    } catch (err) {
      toast.error('Failed to stop service: ' + err);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteService.mutateAsync({ appSlug, serviceId: service.id });
      invalidate();
      setShowDeleteConfirm(false);
    } catch (e) {
      toast.error('Failed to delete service: ' + e);
    }
  };

  const handleRestart = async () => {
    try {
      await restartService.mutateAsync({ appSlug, serviceId: service.id });
      toast.success('Service restart triggered.');
      invalidate();
    } catch (err) {
      toast.error('Failed to restart service: ' + err);
    }
  };

  const handleRedeploy = async () => {
    try {
      await redeployService.mutateAsync({ appSlug, serviceId: service.id });
      toast.success('Service redeploy started.');
      invalidate();
    } catch (err) {
      toast.error('Failed to redeploy service: ' + err);
    }
  };

  const isRunning = service.status === 'Running';
  const canRestart = isRunning || service.status === 'Stopped';
  const canRedeploy =
    isRunning || service.status === 'Stopped' || service.status === 'Failed';

  return (
    <>
      <div className="group grid grid-cols-[1fr_auto] items-center gap-4 px-8 py-4 transition-colors hover:bg-white/[0.02]">
        <VStack space={1.5}>
          <HStack space={3}>
            <span className="font-mono text-[13px] font-semibold text-text">
              {service.name}
            </span>
            <span className="rounded bg-white/5 px-1.5 py-0.5 text-[11px] font-medium text-text-secondary">
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

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button
              type="button"
              aria-label="Service actions"
              className="flex size-7 items-center justify-center rounded-md text-text-tertiary opacity-60 transition-all hover:bg-white/5 hover:text-text group-hover:opacity-100 data-[state=open]:bg-white/5 data-[state=open]:text-text data-[state=open]:opacity-100"
            >
              <MoreHorizontal className="size-4" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={onShowLogs}>
              <Terminal className="size-3.5" />
              Logs
            </DropdownMenuItem>
            {isRunning ? (
              <DropdownMenuItem onClick={() => setShowConnectModal(true)}>
                <Plug className="size-3.5" />
                Connect
              </DropdownMenuItem>
            ) : null}
            <DropdownMenuItem onClick={() => setShowConfig(true)}>
              <Settings className="size-3.5" />
              Configuration
            </DropdownMenuItem>

            <DropdownMenuSeparator />
            {canRestart ? (
              <DropdownMenuItem onClick={handleRestart}>
                <RefreshCw className="size-3.5" />
                Restart
              </DropdownMenuItem>
            ) : null}
            {canRedeploy ? (
              <DropdownMenuItem onClick={handleRedeploy}>
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
      </div>

      <ConfirmationDialog
        open={showDeleteConfirm}
        onOpenChange={setShowDeleteConfirm}
        title="Delete Service"
        description={`Are you sure you want to delete ${service.name}? All underlying data will be permanently destroyed.`}
        confirmLabel="Delete Service"
        onConfirm={handleDelete}
      />

      <ConfirmationDialog
        open={showStopConfirm}
        onOpenChange={setShowStopConfirm}
        title="Stop Service"
        description={`Stop ${service.name}? Apps using it will lose their connection until it is restarted.`}
        confirmLabel="Stop"
        onConfirm={handleStop}
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
