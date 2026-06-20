import { useState } from 'react';
import { useNavigate } from 'react-router';
import { useQueryClient } from '@tanstack/react-query';
import {
  Play,
  Square,
  Terminal,
  Clock,
  AlertCircle,
  CheckCircle2,
  XCircle,
  CircleDashed,
  RotateCcw,
  Trash2,
} from 'lucide-react';
import type { Deployment, DeploymentStatus } from '~/models/deployment';
import {
  useStopDeployment,
  useDeleteDeployment,
  useRestartDeployment,
  useRedeployDeployment,
} from '~/queries/deployments';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import { formatRelativeTime } from '~/utils/format';
import { toast } from 'sonner';

function StatusBadge(props: { status: DeploymentStatus }) {
  const { status } = props;
  const configs: Record<
    DeploymentStatus,
    { icon: any; color: string; bg: string }
  > = {
    Pending: { icon: Clock, color: 'text-text-tertiary', bg: 'bg-white/5' },
    Building: {
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
      <Icon className={cn('size-3', status === 'Building' && 'animate-spin')} />
      {status}
    </span>
  );
}

export function DeploymentRow(props: {
  deployment: Deployment;
  appSlug: string;
  onShowLogs: () => void;
}) {
  const { deployment, appSlug, onShowLogs } = props;
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const stopDeployment = useStopDeployment();
  const deleteDeployment = useDeleteDeployment();
  const restartDeployment = useRestartDeployment();
  const redeployDeployment = useRedeployDeployment();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const handleStop = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await stopDeployment.mutateAsync({
        appSlug,
        deploymentId: deployment.id,
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'deployments'],
      });
    } catch {}
  };

  const handleRestart = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await restartDeployment.mutateAsync({
        appSlug,
        deploymentId: deployment.id,
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'deployments'],
      });
      toast.success('Container started');
    } catch (e) {
      toast.error('Failed to start container: ' + e);
    }
  };

  const handleRedeploy = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await redeployDeployment.mutateAsync({
        appSlug,
        deploymentId: deployment.id,
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'deployments'],
      });
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
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'deployments'],
      });
      setShowDeleteConfirm(false);
    } catch (e) {
      toast.error('Failed to delete deployment: ' + e);
    }
  };

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
            <span className="font-mono text-[12px] font-semibold text-text group-hover:text-primary transition-colors">
              {deployment.commit_sha.slice(0, 7)}
            </span>
            <StatusBadge status={deployment.status} />
            <span className="text-[11px] text-text-tertiary">
              {formatRelativeTime(deployment.created_at)}
            </span>
          </HStack>
          <p className="text-[13px] text-text-secondary line-clamp-1">
            {deployment.commit_message}
          </p>
        </VStack>

        <HStack space={2} onClick={(e) => e.stopPropagation()}>
          <Button
            label="Logs"
            icon={<Terminal className="size-3.5" />}
            variant="ghost"
            size="sm"
            color="neutral"
            onClick={onShowLogs}
          />
          {(deployment.status === 'Running' ||
            deployment.status === 'Building') && (
            <Button
              label="Stop"
              icon={<Square className="size-3.5" />}
              variant="ghost"
              size="sm"
              color="error"
              onClick={handleStop}
              isLoading={stopDeployment.isPending}
            />
          )}
          {(deployment.status === 'Running' ||
            deployment.status === 'Stopped') && (
            <Button
              label="Restart"
              icon={<Play className="size-3.5" />}
              variant="ghost"
              size="sm"
              color="neutral"
              onClick={handleRestart}
              isLoading={restartDeployment.isPending}
            />
          )}
          {(deployment.status === 'Running' ||
            deployment.status === 'Stopped' ||
            deployment.status === 'Failed') && (
            <Button
              label="Redeploy"
              icon={<RotateCcw className="size-3.5" />}
              variant="ghost"
              size="sm"
              color="neutral"
              onClick={handleRedeploy}
              isLoading={redeployDeployment.isPending}
            />
          )}
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
            isLoading={deleteDeployment.isPending}
          />
        </HStack>
      </div>

      <ConfirmationDialog
        open={showDeleteConfirm}
        onOpenChange={setShowDeleteConfirm}
        title="Delete Deployment"
        description="Are you sure you want to delete this deployment? This will also remove the associated Docker container and its logs permanently."
        confirmLabel="Delete"
        onConfirm={handleDelete}
      />
    </>
  );
}
