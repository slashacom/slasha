import { useState } from 'react';
import { useParams, useNavigate } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { ArrowLeft, Check, ChevronRight, Copy, Terminal } from 'lucide-react';
import { toast } from 'sonner';
import { getDeploymentOptions } from '~/queries/deployments';
import { getAppOptions } from '~/queries/apps';
import { HStack, VStack } from '~/components/interface/stacks';
import { StatusBadge } from '~/components/interface/status-badge';
import { LogStream } from '~/components/apps/log-stream';
import { formatRelativeTime, parseUTC } from '~/utils/format';
import { queryClient } from '~/utils/query-client';

type CommitButtonProps = {
  sha: string;
};

function CommitButton(props: CommitButtonProps) {
  const { sha } = props;
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(sha);
      setCopied(true);
      toast.success('Commit SHA copied');
      setTimeout(() => setCopied(false), 1500);
    } catch {}
  };

  return (
    <button
      type="button"
      onClick={handleCopy}
      title={`${sha} — click to copy`}
      className="group inline-flex items-center gap-1.5 rounded font-mono text-[12px] text-text-tertiary transition-colors hover:text-text"
    >
      {sha.slice(0, 7)}
      {copied ? (
        <Check className="size-3 text-emerald-400" />
      ) : (
        <Copy className="size-3 opacity-0 transition-opacity group-hover:opacity-100" />
      )}
    </button>
  );
}

export async function clientLoader(args: {
  params: { slug: string; id: string };
}) {
  const { params } = args;
  await Promise.all([
    queryClient.ensureQueryData(getAppOptions(params.slug)),
    queryClient.ensureQueryData(getDeploymentOptions(params.slug, params.id)),
  ]);
}

function formatDuration(start: string, end: string): string {
  const ms = parseUTC(end).getTime() - parseUTC(start).getTime();
  if (ms < 0) {
    return '—';
  }
  const seconds = Math.round(ms / 1000);
  if (seconds < 60) {
    return `${seconds}s`;
  }
  const minutes = Math.floor(seconds / 60);
  const rem = seconds % 60;
  return `${minutes}m ${rem}s`;
}

type MetaItemProps = {
  label: string;
  children: React.ReactNode;
};

function MetaItem(props: MetaItemProps) {
  const { label, children } = props;
  return (
    <VStack space={1}>
      <span className="text-[11px] font-medium uppercase tracking-wider text-text-tertiary">
        {label}
      </span>
      <div className="text-[13px] text-text">{children}</div>
    </VStack>
  );
}

export default function DeploymentDetailPage() {
  const { slug, id } = useParams();
  const navigate = useNavigate();

  const { data: appData } = useSuspenseQuery(getAppOptions(slug!));
  const { data: deploymentData } = useSuspenseQuery(
    getDeploymentOptions(slug!, id!)
  );

  const app = appData.app;
  const deployment = deploymentData.deployment;
  const isTerminal =
    deployment.status === 'Running' ||
    deployment.status === 'Failed' ||
    deployment.status === 'Stopped';

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-bg">
      <HStack
        justifyContent="between"
        alignItems="center"
        className="shrink-0 gap-4 border-b border-border bg-surface/30 px-8 py-3"
      >
        <HStack space={3} alignItems="center">
          <button
            onClick={() => navigate(`/apps/${slug}/deployments`)}
            className="group flex size-7 items-center justify-center rounded border border-border bg-surface transition-all hover:bg-white/[0.06]"
          >
            <ArrowLeft className="size-3.5 text-text-tertiary group-hover:text-text" />
          </button>
          <HStack space={2} alignItems="center">
            <span className="text-[13px] font-medium text-text">
              {app.name}
            </span>
            <ChevronRight className="size-3 text-text-tertiary" />
            <CommitButton sha={deployment.commit_sha} />
          </HStack>
        </HStack>

        <HStack space={3} alignItems="center">
          <StatusBadge status={deployment.status} />
          <span className="text-[11px] text-text-tertiary">
            Deployed {formatRelativeTime(deployment.created_at)}
          </span>
        </HStack>
      </HStack>

      <div className="flex min-h-0 flex-1 flex-col gap-6 p-8">
        <div className="grid grid-cols-2 gap-6 rounded-lg border border-border bg-surface/30 p-6 sm:grid-cols-4">
          <MetaItem label="Commit">
            <CommitButton sha={deployment.commit_sha} />
          </MetaItem>
          <MetaItem label="Status">
            <StatusBadge status={deployment.status} />
          </MetaItem>
          <MetaItem label="Created">
            {formatRelativeTime(deployment.created_at)}
          </MetaItem>
          <MetaItem label="Duration">
            {isTerminal
              ? formatDuration(deployment.created_at, deployment.updated_at)
              : 'In progress…'}
          </MetaItem>
          <div className="col-span-2 sm:col-span-4">
            <MetaItem label="Message">
              <span className="text-text-secondary">
                {deployment.commit_message || '—'}
              </span>
            </MetaItem>
          </div>
        </div>

        <VStack space={3} className="flex min-h-0 flex-1 flex-col">
          <HStack space={2} alignItems="center">
            <Terminal className="size-4 text-text-tertiary" />
            <h3 className="text-sm font-semibold text-text">
              Build & runtime logs
            </h3>
          </HStack>
          <LogStream
            url={`/api/apps/${slug}/deployments/${id}/logs`}
            className="min-h-0 flex-1 rounded-lg border border-border"
          />
        </VStack>
      </div>
    </div>
  );
}
