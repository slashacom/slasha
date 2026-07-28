import { useState } from 'react';
import { useNavigate } from 'react-router';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Play,
  History,
  RotateCcw,
  Copy,
  Check,
  ChevronDown,
  GitCommit,
  CircleDashed,
} from 'lucide-react';
import {
  getCommitsOptions,
  getDeploymentsOptions,
  useTriggerDeploy,
} from '~/queries/deployments';
import { SectionHeader } from '~/components/interface/section-header';
import { VStack } from '~/components/interface/stacks';
import { toast } from 'sonner';
import { CommitSelector } from '~/components/apps/commit-selector';
import { DeploymentRow } from '~/components/apps/deployment-row';
import type { App } from '~/models/app';

import { Button } from '~/components/interface/button';

type DeploymentsViewProps = {
  app: App;
};

function GitSetupInstructions(props: {
  cloneUrl: string;
  defaultBranch: string;
}) {
  const { cloneUrl, defaultBranch } = props;
  const [copied, setCopied] = useState(false);

  const commandText = `git add .\ngit commit -m "initial commit"\ngit branch -M ${defaultBranch}\ngit remote add slasha ${cloneUrl}\ngit push -u slasha ${defaultBranch}`;

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(commandText);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
      toast.success('Copied git commands to clipboard');
    } catch (e) {
      toast.error('Failed to copy: ' + e);
    }
  };

  return (
    <div className="w-full max-w-xl text-left overflow-hidden rounded-xl border border-border bg-surface/40">
      <div className="flex items-center justify-between border-b border-border bg-white/[0.02] px-4 py-2.5">
        <span className="text-xs font-semibold text-text">
          Push an existing repository from the command line
        </span>
        <button
          type="button"
          onClick={handleCopy}
          aria-label="Copy commands"
          className="cursor-pointer rounded p-1 text-text-tertiary transition-colors hover:bg-white/10 hover:text-text"
        >
          {copied ? (
            <Check className="size-3.5 text-emerald-400" />
          ) : (
            <Copy className="size-3.5" />
          )}
        </button>
      </div>
      <pre className="overflow-x-auto bg-black/50 p-4 font-mono text-[12px] leading-relaxed text-text-secondary">
        <span className="select-none text-text-tertiary">$ </span>git add .
        {'\n'}
        <span className="select-none text-text-tertiary">$ </span>git commit -m
        &quot;initial commit&quot;{'\n'}
        <span className="select-none text-text-tertiary">$ </span>git branch -M{' '}
        <span className="text-text">{defaultBranch}</span>
        {'\n'}
        <span className="select-none text-text-tertiary">$ </span>git remote add
        slasha <span className="text-text">{cloneUrl}</span>
        {'\n'}
        <span className="select-none text-text-tertiary">$ </span>git push -u
        slasha <span className="text-text">{defaultBranch}</span>
      </pre>
    </div>
  );
}

export function DeploymentsView(props: DeploymentsViewProps) {
  const { app } = props;
  const appSlug = app.slug;
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
  const { data: commitsData } = useQuery(getCommitsOptions(appSlug));
  const hasCode = (commitsData?.commits?.length ?? 0) > 0;

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
              isDisabled={!hasCode || triggerDeploy.isPending}
            />
            <Button
              label="Deploy Latest"
              icon={<Play className="size-3.5" />}
              size="sm"
              onClick={handleDeploy}
              isLoading={triggerDeploy.isPending}
              isDisabled={!hasCode || triggerDeploy.isPending}
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
              {app.source === 'local'
                ? 'Add the remote and push to deploy your default branch.'
                : app.source === 'github'
                  ? 'Push to the connected GitHub repository for automatic deployments, or deploy the latest commit now.'
                  : 'Deploy the latest commit from the configured Git repository.'}
            </p>
          </VStack>

          {app.source === 'local' && (
            <GitSetupInstructions
              cloneUrl={cloneUrl}
              defaultBranch={app.default_branch}
            />
          )}
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
