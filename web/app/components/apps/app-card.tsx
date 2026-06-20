import { Link } from 'react-router';
import { GitBranchIcon } from 'lucide-react';
import { cn } from '~/utils/classname';
import type { App } from '~/models/app';

export function AppCard(props: { app: App }) {
  const { app } = props;
  return (
    <Link
      to={`/apps/${app.slug}`}
      className="group block rounded-lg border border-border bg-surface p-4 !no-underline transition-colors hover:bg-white/[0.04]"
    >
      <div className="flex items-start justify-between">
        <div className="min-w-0">
          <h3 className="truncate text-[14px] font-medium text-text">
            {app.name}
          </h3>
          <p className="mt-0.5 truncate font-mono text-[12px] text-text-tertiary">
            {app.slug}
          </p>
        </div>
        <span
          className={cn(
            'rounded px-1.5 py-0.5 text-[11px] font-medium',
            app.status === 'active'
              ? 'bg-emerald-900/30 text-emerald-400'
              : 'bg-white/5 text-text-tertiary'
          )}
        >
          {app.status}
        </span>
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
