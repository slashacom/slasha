import { Link } from 'react-router';
import { ArrowUpRight, GitBranchIcon } from 'lucide-react';
import type { AppListItem } from '~/queries/apps';
import { AppRuntimeBadge } from '~/components/apps/app-runtime-badge';
import { statusFromRuntime } from '~/utils/app-status';

type AppCardProps = {
  item: AppListItem;
};

export function AppCard(props: AppCardProps) {
  const { item } = props;
  const { app, url, runtime_status } = item;
  const status = statusFromRuntime(runtime_status);

  return (
    <Link
      to={`/apps/${app.slug}`}
      className="group block rounded-lg border border-border bg-surface p-4 !no-underline transition-colors hover:bg-white/[0.04]"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <h3 className="truncate text-[14px] font-medium text-text">
            {app.name}
          </h3>
          <p className="mt-0.5 truncate font-mono text-[12px] text-text-tertiary">
            {app.slug}
          </p>
        </div>
        <AppRuntimeBadge status={status} />
      </div>

      <div className="mt-4 flex items-center justify-between text-[12px] text-text-tertiary">
        <div className="flex items-center gap-1.5">
          <GitBranchIcon className="size-3.5" />
          <span>{app.default_branch}</span>
          <span className="px-1">·</span>
          <span>{new Date(app.created_at).toLocaleDateString()}</span>
        </div>
        {status.tone === 'live' ? (
          <a
            href={url}
            target="_blank"
            rel="noreferrer"
            onClick={(e) => e.stopPropagation()}
            className="inline-flex items-center gap-0.5 text-text-tertiary !no-underline opacity-0 transition-all hover:text-text group-hover:opacity-100"
          >
            Visit
            <ArrowUpRight className="size-3" />
          </a>
        ) : null}
      </div>
    </Link>
  );
}
