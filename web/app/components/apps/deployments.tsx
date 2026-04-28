import { useEffect, useRef, useState } from 'react';
import { useParams } from 'react-router';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Activity,
  ExternalLink,
  Play,
  Square,
  Terminal,
  Clock,
  History,
  AlertCircle,
  CheckCircle2,
  XCircle,
  CircleDashed,
  RotateCcw,
  Trash2,
  Search,
} from 'lucide-react';
import type { Deployment, DeploymentStatus } from '~/models/deployment';
import {
  getDeploymentsOptions,
  getCommitsOptions,
  useTriggerDeploy,
  useStopDeployment,
  useDeleteDeployment,
  useRestartDeployment,
  type CommitInfo,
} from '~/queries/deployments';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '~/components/interface/dialog';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import { formatRelativeTime } from '~/utils/format';
import { getAuthToken } from '~/utils/jwt';
import { toast } from 'sonner';

export function DeploymentsView({ appSlug }: { appSlug: string }) {
  const { data, isLoading } = useQuery(getDeploymentsOptions(appSlug));
  const triggerDeploy = useTriggerDeploy();
  const queryClient = useQueryClient();
  const [activeLogsId, setActiveLogsId] = useState<string | null>(null);
  const [showCommitSelector, setShowCommitSelector] = useState(false);

  const deployments = data?.deployments ?? [];

  const handleDeploy = async () => {
    try {
      await triggerDeploy.mutateAsync({ appSlug });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'deployments'],
      });
    } catch (e) {
      toast.error('Failed to trigger deploy: ' + e);
    }
  };

  if (isLoading) {
    return (
      <VStack className="p-8" space={4}>
        <div className="h-4 w-32 animate-pulse rounded bg-surface-hover" />
        <VStack space={2}>
          {[1, 2, 3].map((i) => (
            <div
              key={i}
              className="h-16 w-full animate-pulse rounded border border-border bg-surface"
            />
          ))}
        </VStack>
      </VStack>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-hidden">
      <HStack
        justifyContent="between"
        className="border-b border-border px-8 py-4"
      >
        <HStack space={2}>
          <History className="size-4 text-text-tertiary" />
          <h2 className="text-sm font-semibold text-text">
            Deployment History
          </h2>
        </HStack>
        <HStack space={2}>
          <Button
            label="Deploy Commit"
            variant="ghost"
            size="sm"
            onClick={() => setShowCommitSelector(true)}
          />
          <Button
            label="Deploy Latest"
            icon={<Play className="size-3.5" />}
            size="sm"
            onClick={handleDeploy}
            isLoading={triggerDeploy.isPending}
          />
        </HStack>
      </HStack>

      {deployments.length === 0 ? (
        <VStack className="flex-1 items-center justify-center" space={4}>
          <div className="rounded-full border border-border p-4">
            <RotateCcw className="size-8 text-text-tertiary" />
          </div>
          <VStack alignItems="center" space={1}>
            <p className="text-sm font-medium text-text">No deployments yet</p>
            <p className="text-xs text-text-tertiary text-center max-w-[280px]">
              Deployments will appear here once you trigger a build or push
              code.
            </p>
          </VStack>
          <Button
            label="Trigger First Deployment"
            size="sm"
            onClick={handleDeploy}
            isLoading={triggerDeploy.isPending}
          />
        </VStack>
      ) : (
        <div className="flex-1 overflow-auto">
          <div className="divide-y divide-border">
            {deployments.map((deployment) => (
              <DeploymentRow
                key={deployment.id}
                deployment={deployment}
                appSlug={appSlug}
                onShowLogs={() => setActiveLogsId(deployment.id)}
              />
            ))}
          </div>
        </div>
      )}

      {activeLogsId && (
        <LogModal
          deploymentId={activeLogsId}
          appSlug={appSlug}
          onClose={() => setActiveLogsId(null)}
        />
      )}

      <CommitSelector
        open={showCommitSelector}
        onOpenChange={setShowCommitSelector}
        appSlug={appSlug}
        onSelect={async (sha) => {
          try {
            await triggerDeploy.mutateAsync({ appSlug, commitSha: sha });
            queryClient.invalidateQueries({
              queryKey: ['apps', appSlug, 'deployments'],
            });
            setShowCommitSelector(false);
            toast.success('Deployment triggered for ' + sha.slice(0, 7));
          } catch (e) {
            toast.error('Failed to trigger deploy: ' + e);
          }
        }}
        isDeploying={triggerDeploy.isPending}
      />
    </div>
  );
}

function CommitSelector({
  open,
  onOpenChange,
  appSlug,
  onSelect,
  isDeploying,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  appSlug: string;
  onSelect: (sha: string) => void;
  isDeploying: boolean;
}) {
  const { data, isLoading } = useQuery(getCommitsOptions(appSlug));
  const [search, setSearch] = useState('');
  const commits = data?.commits ?? [];

  const filteredCommits = commits.filter(
    (c) =>
      c.message.toLowerCase().includes(search.toLowerCase()) ||
      c.sha.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px] p-0 gap-0 overflow-hidden">
        <DialogHeader className="p-6 border-b border-border pb-4">
          <DialogTitle>Deploy Specific Commit</DialogTitle>
          <DialogDescription>
            Select a commit to trigger a new deployment.
          </DialogDescription>
        </DialogHeader>

        <div className="px-6 py-4 border-b border-border">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
            <Input
              placeholder="Search by message or SHA..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9 bg-surface"
              autoFocus
            />
          </div>
        </div>

        <div className="max-h-[400px] overflow-y-auto">
          {isLoading ? (
            <VStack className="p-8 items-center" space={4}>
              <CircleDashed className="size-6 animate-spin text-text-tertiary" />
              <p className="text-xs text-text-tertiary">Fetching commits...</p>
            </VStack>
          ) : filteredCommits.length === 0 ? (
            <VStack className="p-8 items-center" space={2}>
              <p className="text-sm text-text-secondary">No commits found</p>
              {search && (
                <p className="text-xs text-text-tertiary">
                  Try adjusting your search for "{search}"
                </p>
              )}
            </VStack>
          ) : (
            <div className="divide-y divide-border">
              {filteredCommits.map((commit) => (
                <button
                  key={commit.sha}
                  onClick={() => onSelect(commit.sha)}
                  disabled={isDeploying}
                  className="w-full text-left px-6 py-3 hover:bg-white/[0.02] transition-colors disabled:opacity-50 group"
                >
                  <VStack space={1}>
                    <HStack space={2} alignItems="center">
                      <span className="font-mono text-[12px] font-semibold text-text group-hover:text-primary transition-colors">
                        {commit.sha.slice(0, 7)}
                      </span>
                    </HStack>
                    <p className="text-[13px] text-text-secondary line-clamp-1">
                      {commit.message}
                    </p>
                  </VStack>
                </button>
              ))}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function StatusBadge({ status }: { status: DeploymentStatus }) {
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

function DeploymentRow({
  deployment,
  appSlug,
  onShowLogs,
}: {
  deployment: Deployment;
  appSlug: string;
  onShowLogs: () => void;
}) {
  const queryClient = useQueryClient();
  const stopDeployment = useStopDeployment();
  const deleteDeployment = useDeleteDeployment();
  const restartDeployment = useRestartDeployment();
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
      toast.success('Deployment restart triggered');
    } catch (e) {
      toast.error('Failed to restart deployment: ' + e);
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
      <div className="group grid grid-cols-[1fr_auto] items-center gap-4 px-8 py-4 transition-colors hover:bg-white/[0.02]">
        <VStack space={1.5}>
          <HStack space={3}>
            <span className="font-mono text-[12px] font-semibold text-text">
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

        <HStack space={2}>
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
          {(deployment.status === 'Stopped' ||
            deployment.status === 'Failed') && (
            <Button
              label="Restart"
              icon={<RotateCcw className="size-3.5" />}
              variant="ghost"
              size="sm"
              color="neutral"
              onClick={handleRestart}
              isLoading={restartDeployment.isPending}
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

function LogModal({
  deploymentId,
  appSlug,
  onClose,
}: {
  deploymentId: string;
  appSlug: string;
  onClose: () => void;
}) {
  const [logs, setLogs] = useState<string[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const token = getAuthToken();
    const url = `/api/apps/${appSlug}/deployments/${deploymentId}/logs?token=${token}`;
    const es = new EventSource(url);

    es.onmessage = (event) => {
      const data = event.data;
      if (data) {
        setLogs((prev) => [...prev, data]);
      }
    };

    es.onerror = (e) => {
      console.error('SSE Stream error:', e);
    };

    return () => {
      es.close();
    };
  }, [appSlug, deploymentId]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 px-4 py-8 backdrop-blur-sm">
      <div
        ref={containerRef}
        className="flex h-full w-full max-w-4xl flex-col rounded-lg border border-border bg-bg shadow-2xl"
      >
        <HStack
          justifyContent="between"
          className="shrink-0 border-b border-border p-4"
        >
          <HStack space={3}>
            <Terminal className="size-4 text-text-tertiary" />
            <h3 className="text-sm font-semibold text-text">Logs</h3>
          </HStack>
          <Button label="Close" variant="ghost" size="sm" onClick={onClose} />
        </HStack>

        <div
          ref={scrollRef}
          className="flex-1 overflow-auto bg-black/40 p-6 font-mono text-[13px] leading-relaxed selection:bg-white/10"
        >
          {logs.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full gap-3 text-text-tertiary">
              <CircleDashed className="size-5 animate-spin" />
              <p>Establishing log stream...</p>
            </div>
          ) : (
            <div className="space-y-1">
              {logs.map((log, i) => (
                <div
                  key={i}
                  className="text-text-secondary whitespace-pre-wrap break-all"
                >
                  <span className="text-text-tertiary mr-3 select-none">
                    {i + 1}
                  </span>
                  {log}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
