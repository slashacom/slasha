import { useState } from 'react';
import { useNavigate } from 'react-router';
import {
  Eye,
  MoreHorizontal,
  Plug,
  RefreshCw,
  RotateCcw,
  Square,
  Trash2,
} from 'lucide-react';
import { toast } from 'sonner';
import type { Service } from '~/models/service';
import {
  useRestartService,
  useRedeployService,
  useStopService,
  useDeleteService,
} from '~/queries/services';
import { ConnectModal } from '~/components/apps/connect-modal';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '~/components/interface/dropdown-menu';

type ServiceActionsMenuProps = {
  appSlug: string;
  service: Service;
  variant?: 'panel' | 'row';
};

export function ServiceActionsMenu(props: ServiceActionsMenuProps) {
  const { appSlug, service, variant = 'panel' } = props;
  const navigate = useNavigate();
  const restartService = useRestartService();
  const redeployService = useRedeployService();
  const stopService = useStopService();
  const deleteService = useDeleteService();
  const [showConnect, setShowConnect] = useState(false);
  const [showRestartConfirm, setShowRestartConfirm] = useState(false);
  const [showRedeployConfirm, setShowRedeployConfirm] = useState(false);
  const [showStopConfirm, setShowStopConfirm] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const handleRestart = async () => {
    try {
      await restartService.mutateAsync({ appSlug, serviceId: service.id });
      toast.success('Service restart triggered.');
      setShowRestartConfirm(false);
    } catch (err) {
      toast.error('Failed to restart service: ' + err);
    }
  };

  const handleRedeploy = async () => {
    try {
      await redeployService.mutateAsync({ appSlug, serviceId: service.id });
      toast.success('Service redeploy started.');
      setShowRedeployConfirm(false);
    } catch (err) {
      toast.error('Failed to redeploy service: ' + err);
    }
  };

  const handleStop = async () => {
    try {
      await stopService.mutateAsync({ appSlug, serviceId: service.id });
      setShowStopConfirm(false);
    } catch (err) {
      toast.error('Failed to stop service: ' + err);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteService.mutateAsync({ appSlug, serviceId: service.id });
      setShowDeleteConfirm(false);
      navigate(`/apps/${appSlug}/services`);
    } catch (err) {
      toast.error('Failed to delete service: ' + err);
    }
  };

  const isRunning = service.status === 'Running';
  const canRestart = isRunning || service.status === 'Stopped';
  const canRedeploy =
    isRunning || service.status === 'Stopped' || service.status === 'Failed';

  const triggerClasses =
    variant === 'row'
      ? 'flex size-7 items-center justify-center rounded-md text-text-tertiary opacity-60 transition-all hover:bg-white/5 hover:text-text group-hover:opacity-100 data-[state=open]:bg-white/5 data-[state=open]:text-text data-[state=open]:opacity-100'
      : 'flex size-7 items-center justify-center rounded-md border border-border bg-surface text-text-tertiary transition-all hover:bg-white/[0.06] hover:text-text data-[state=open]:bg-white/[0.06] data-[state=open]:text-text';

  return (
    <div onClick={(event) => event.stopPropagation()}>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            aria-label="Service actions"
            className={triggerClasses}
          >
            <MoreHorizontal className="size-4" />
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {variant === 'row' ? (
            <DropdownMenuItem
              onClick={() =>
                navigate(`/apps/${appSlug}/services/${service.id}`)
              }
            >
              <Eye className="size-3.5" />
              View service
            </DropdownMenuItem>
          ) : null}
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

      {showConnect ? (
        <ConnectModal
          appSlug={appSlug}
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
