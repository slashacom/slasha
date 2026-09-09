import { useNavigate } from 'react-router';
import { ArrowUpRight, GitBranch } from 'lucide-react';
import type { AppSource } from '~/models/app';
import type { AppListItem } from '~/queries/apps';
import { AppRuntimeBadge } from '~/components/apps/app-runtime-badge';
import { HStack, VStack } from '~/components/interface/stacks';
import { formatRelativeTime } from '~/utils/date';

const SOURCE_LABELS: Record<AppSource, string> = {
  local: 'Slasha Git',
  github: 'GitHub',
  git: 'Git URL',
};

type AppCardProps = {
  item: AppListItem;
};

export function AppCard(props: AppCardProps) {
  const { item } = props;
  const { app, url, runtime_status } = item;
  const navigate = useNavigate();

  return (
    <div
      role="link"
      tabIndex={0}
      onClick={() => navigate(`/apps/${app.slug}`)}
      onKeyDown={(event) => {
        if (event.key !== 'Enter') {
          return;
        }

        navigate(`/apps/${app.slug}`);
      }}
      className="group cursor-pointer rounded-lg border border-border bg-surface/60 p-4 transition-colors hover:bg-surface focus-visible:bg-surface focus-visible:outline-none"
    >
      <HStack justifyContent="between" alignItems="start" className="gap-2">
        <VStack space={0.5} className="min-w-0">
          <span className="truncate text-[14px] font-medium text-text">
            {app.name}
          </span>
          <code className="truncate font-mono text-[12px] text-text-tertiary">
            {app.slug}
          </code>
        </VStack>
        <AppRuntimeBadge status={runtime_status} />
      </HStack>

      <HStack
        justifyContent="between"
        className="mt-4 gap-2 border-t border-border/60 pt-3"
      >
        <HStack space={1.5} className="min-w-0">
          <GitBranch className="size-3 shrink-0 text-text-tertiary" />
          <span className="truncate text-[11px] text-text-tertiary">
            {app.default_branch}
          </span>
          <span className="text-[11px] text-text-tertiary/50">·</span>
          <span className="truncate text-[11px] text-text-tertiary">
            {SOURCE_LABELS[app.source]}
          </span>
        </HStack>

        {runtime_status === 'running' ? (
          <a
            href={url}
            target="_blank"
            rel="noreferrer"
            onClick={(event) => event.stopPropagation()}
            className="inline-flex shrink-0 items-center gap-1 text-[11px] font-medium text-text-tertiary !no-underline transition-colors hover:text-text"
          >
            Visit
            <ArrowUpRight className="size-3" />
          </a>
        ) : (
          <span className="shrink-0 whitespace-nowrap text-[11px] text-text-tertiary/70">
            {formatRelativeTime(app.created_at)}
          </span>
        )}
      </HStack>
    </div>
  );
}
