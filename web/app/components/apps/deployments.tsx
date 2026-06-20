import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Play, History, RotateCcw } from 'lucide-react';
import { getDeploymentsOptions, useTriggerDeploy } from '~/queries/deployments';
import { Button } from '~/components/interface/button';
import { SectionHeader } from '~/components/interface/section-header';
import { VStack } from '~/components/interface/stacks';
import { toast } from 'sonner';
import { CommitSelector } from '~/components/apps/commit-selector';
import { DeploymentRow } from '~/components/apps/deployment-row';
import { LogModal } from '~/components/apps/deployment-log-modal';

type DeploymentsViewProps = {
  appSlug: string;
};

export function DeploymentsView(props: DeploymentsViewProps) {
  const { appSlug } = props;
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
      <SectionHeader
        icon={History}
        title="Deployment History"
        actions={
          <>
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
          </>
        }
      />

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
