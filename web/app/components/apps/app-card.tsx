import { Link } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import { GitBranchIcon } from 'lucide-react';
import type { App } from '~/models/app';
import { getDeploymentsOptions } from '~/queries/deployments';
import { AppRuntimeBadge } from '~/components/apps/app-runtime-badge';
import { deriveAppStatus } from '~/utils/app-status';

type AppCardProps = {
  app: App;
};

export function AppCard(props: AppCardProps) {
  const { app } = props;
  const { data } = useQuery(getDeploymentsOptions(app.slug));
  const status = deriveAppStatus(data?.deployments ?? []);

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

      <div className="mt-4 flex items-center gap-1.5 text-[12px] text-text-tertiary">
        <GitBranchIcon className="size-3.5" />
        <span>{app.default_branch}</span>
        <span className="px-1">·</span>
        <span>{new Date(app.created_at).toLocaleDateString()}</span>
      </div>
    </Link>
  );
}
