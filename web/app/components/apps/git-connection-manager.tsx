import { ExternalLink, GitBranch, Link as LinkIcon } from 'lucide-react';
import type { App } from '~/models/app';
import type { GitAppConnection } from '~/queries/apps';

type Props = {
  app: App;
  connection?: GitAppConnection;
};

export function GitConnectionManager({ app, connection }: Props) {
  if (app.source !== 'git' || !connection) {
    return null;
  }

  return (
    <div>
      <h3 className="text-[14px] font-semibold text-text">Git Repository</h3>
      <p className="mt-1 text-[13px] text-text-tertiary">
        Source repository mirrored by this application.
      </p>

      <div className="mt-6 border border-border bg-surface p-6">
        <div className="grid gap-5 sm:grid-cols-2">
          <div className="min-w-0">
            <div className="flex items-center gap-2 text-[12px] text-text-tertiary">
              <LinkIcon className="size-3.5" />
              Repository
            </div>
            <a
              href={connection.clone_url}
              target="_blank"
              rel="noreferrer"
              className="mt-1.5 flex min-w-0 items-center gap-1.5 text-[13px] font-medium text-text hover:underline"
            >
              <span className="truncate">{connection.clone_url}</span>
              <ExternalLink className="size-3.5 shrink-0 text-text-tertiary" />
            </a>
          </div>
          <div>
            <div className="flex items-center gap-2 text-[12px] text-text-tertiary">
              <GitBranch className="size-3.5" />
              Branch
            </div>
            <p className="mt-1.5 font-mono text-[13px] font-medium text-text">
              {app.default_branch}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
