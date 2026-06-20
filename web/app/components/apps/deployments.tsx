import { useState } from 'react';
import { useNavigate } from 'react-router';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Play, History, RotateCcw } from 'lucide-react';
import { getDeploymentsOptions, useTriggerDeploy } from '~/queries/deployments';
import { Button } from '~/components/interface/button';
import { SectionHeader } from '~/components/interface/section-header';
import { VStack } from '~/components/interface/stacks';
import { toast } from 'sonner';
import { CommitSelector } from '~/components/apps/commit-selector';
import { DeploymentRow } from '~/components/apps/deployment-row';

type DeploymentsViewProps = {
  appSlug: string;
};

export function DeploymentsView(props: DeploymentsViewProps) {
  const { appSlug } = props;
  const navigate = useNavigate();
  const { data, isLoading } = useQuery({
    ...getDeploymentsOptions(appSlug),
    refetchInterval: (query) => {
      const deps = query.state.data?.deployments ?? [];
      const active = deps.some(
        (d) => d.status === 'Building' || d.status === 'Pending'
      );
      return active ? 2000 : false;
    },
  });
  const triggerDeploy = useTriggerDeploy();
  const queryClient = useQueryClient();
  const [showCommitSelector, setShowCommitSelector] = useState(false);

  const deployments = data?.deployments ?? [];
  const cloneUrl =
    typeof window === 'undefined'
      ? `/git/${appSlug}`
      : `${window.location.origin}/git/${appSlug}`;

  const handleDeploy = async () => {
    try {
      const result = await triggerDeploy.mutateAsync({ appSlug });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'deployments'],
      });
      navigate(`/apps/${appSlug}/deployments/${result.deployment.id}`);
    } catch (e) {
      toast.error('Failed to trigger deploy: ' + e);
    }
  };

  if (isLoading) {
    return (
      <VStack className="p-8" space={4}>
        <div className="h-4 w-32 animate-pulse rounded bg-white/[0.06]" />
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
        title="Deployments"
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
        <VStack className="flex-1 items-center justify-center" space={5}>
          <div className="rounded-full border border-border p-4">
            <RotateCcw className="size-8 text-text-tertiary" />
          </div>
          <VStack alignItems="center" space={1}>
            <p className="text-sm font-medium text-text">No deployments yet</p>
            <p className="max-w-[340px] text-center text-xs text-text-tertiary">
              Add the remote and push — slasha deploys every push to your
              default branch automatically. Or trigger one now.
            </p>
          </VStack>
          <pre className="w-full max-w-md overflow-x-auto rounded-lg border border-border bg-black/40 p-4 text-left font-mono text-[12px] leading-relaxed text-text-secondary">
            <span className="select-none text-text-tertiary">$ </span>git remote
            add slasha {cloneUrl}
            {'\n'}
            <span className="select-none text-text-tertiary">$ </span>git push
            slasha main
          </pre>
          <Button
            label="Deploy now"
            icon={<Play className="size-3.5" />}
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
                isCurrent={deployment.status === 'Running'}
              />
            ))}
          </div>
        </div>
      )}

      <CommitSelector
        open={showCommitSelector}
        onOpenChange={setShowCommitSelector}
        appSlug={appSlug}
        onSelect={async (sha) => {
          try {
            const result = await triggerDeploy.mutateAsync({
              appSlug,
              commitSha: sha,
            });
            queryClient.invalidateQueries({
              queryKey: ['apps', appSlug, 'deployments'],
            });
            setShowCommitSelector(false);
            navigate(`/apps/${appSlug}/deployments/${result.deployment.id}`);
          } catch (e) {
            toast.error('Failed to trigger deploy: ' + e);
          }
        }}
        isDeploying={triggerDeploy.isPending}
      />
    </div>
  );
}
