import { useState } from 'react';
import { useNavigate } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import { MoreHorizontal, Play, RotateCcw, Square, Trash2 } from 'lucide-react';
import type { Deployment } from '~/models/deployment';
import {
  useStopDeployment,
  useDeleteDeployment,
  useRestartDeployment,
  useRedeployDeployment,
} from '~/queries/deployments';
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

type DeploymentRowProps = {
  deployment: Deployment;
  appSlug: string;
  isCurrent?: boolean;
};

export function DeploymentRow(props: DeploymentRowProps) {
  const { deployment, appSlug, isCurrent = false } = props;
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const stopDeployment = useStopDeployment();
  const deleteDeployment = useDeleteDeployment();
  const restartDeployment = useRestartDeployment();
  const redeployDeployment = useRedeployDeployment();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [showStopConfirm, setShowStopConfirm] = useState(false);

  const invalidate = () => {
    queryClient.invalidateQueries({
      queryKey: ['apps', appSlug, 'deployments'],
    });
  };

  const handleStop = async () => {
    try {
      await stopDeployment.mutateAsync({
        appSlug,
        deploymentId: deployment.id,
      });
      invalidate();
      setShowStopConfirm(false);
    } catch {}
  };

  const handleRestart = async () => {
    try {
      await restartDeployment.mutateAsync({
        appSlug,
        deploymentId: deployment.id,
      });
      invalidate();
      toast.success('Container started');
    } catch (e) {
      toast.error('Failed to start container: ' + e);
    }
  };

  const handleRedeploy = async () => {
    try {
      await redeployDeployment.mutateAsync({
        appSlug,
        deploymentId: deployment.id,
      });
      invalidate();
      toast.success('Redeploy triggered');
    } catch (e) {
      toast.error('Failed to redeploy: ' + e);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteDeployment.mutateAsync({
        appSlug,
        deploymentId: deployment.id,
      });
      invalidate();
      setShowDeleteConfirm(false);
    } catch (e) {
      toast.error('Failed to delete deployment: ' + e);
    }
  };

  const canStop =
    deployment.status === 'Running' || deployment.status === 'Building';
  const canRestart =
    deployment.status === 'Running' || deployment.status === 'Stopped';
  const canRedeploy =
    deployment.status === 'Running' ||
    deployment.status === 'Stopped' ||
    deployment.status === 'Failed';
  const canDelete = deployment.status !== 'Building' && !isCurrent;

  return (
    <>
      <div
        onClick={() =>
          navigate(`/apps/${appSlug}/deployments/${deployment.id}`)
        }
        className="group grid cursor-pointer grid-cols-[1fr_auto] items-center gap-4 px-8 py-4 transition-colors hover:bg-white/[0.02]"
      >
        <VStack space={1.5}>
          <HStack space={3}>
            <span className="font-mono text-[12px] font-semibold text-text transition-colors group-hover:text-primary">
              {deployment.commit_sha.slice(0, 7)}
            </span>
            <StatusBadge status={deployment.status} />
            {isCurrent ? (
              <span className="rounded border border-emerald-500/20 bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-emerald-400">
                Current
              </span>
            ) : null}
            <span className="text-[11px] text-text-tertiary">
              {formatRelativeTime(deployment.created_at)}
            </span>
          </HStack>
          <p className="line-clamp-1 text-[13px] text-text-secondary">
            {deployment.commit_message}
          </p>
        </VStack>

        <div onClick={(e) => e.stopPropagation()}>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                aria-label="Deployment actions"
                className="flex size-7 items-center justify-center rounded-md text-text-tertiary opacity-60 transition-all hover:bg-white/5 hover:text-text group-hover:opacity-100 data-[state=open]:bg-white/5 data-[state=open]:text-text data-[state=open]:opacity-100"
              >
                <MoreHorizontal className="size-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              {canStop ? (
                <DropdownMenuItem onClick={() => setShowStopConfirm(true)}>
                  <Square className="size-3.5" />
                  Stop
                </DropdownMenuItem>
              ) : null}
              {canRestart ? (
                <DropdownMenuItem onClick={handleRestart}>
                  <Play className="size-3.5" />
                  Restart
                </DropdownMenuItem>
              ) : null}
              {canRedeploy ? (
                <DropdownMenuItem onClick={handleRedeploy}>
                  <RotateCcw className="size-3.5" />
                  Redeploy
                </DropdownMenuItem>
              ) : null}
              {canDelete ? (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    variant="destructive"
                    onClick={() => setShowDeleteConfirm(true)}
                  >
                    <Trash2 className="size-3.5" />
                    Delete
                  </DropdownMenuItem>
                </>
              ) : null}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      <ConfirmationDialog
        open={showDeleteConfirm}
        onOpenChange={setShowDeleteConfirm}
        title="Delete Deployment"
        description="Are you sure you want to delete this deployment? This will also remove the associated Docker container and its logs permanently."
        confirmLabel="Delete"
        onConfirm={handleDelete}
      />

      <ConfirmationDialog
        open={showStopConfirm}
        onOpenChange={setShowStopConfirm}
        title="Stop Deployment"
        description="This stops the running containers and takes the app offline until you restart or redeploy it."
        confirmLabel="Stop"
        onConfirm={handleStop}
      />
    </>
  );
}
