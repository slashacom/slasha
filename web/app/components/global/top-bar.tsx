import { useMemo, useState } from 'react';
import { Link, useLocation, useParams } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import {
  ArrowUpRight,
  Check,
  ChevronRight,
  Copy,
  GitBranch,
} from 'lucide-react';
import { AppRuntimeBadge } from '~/components/apps/app-runtime-badge';
import { NodeStatusBadge } from '~/components/interface/status-badge';
import { HStack } from '~/components/interface/stacks';
import { getAppOptions, type AppRuntimeStatus } from '~/queries/apps';
import { getNodeOptions } from '~/queries/nodes';
import type { App } from '~/models/app';
import { cn } from '~/utils/classname';

type Crumb = {
  label: string;
  to?: string;
};

const chipClasses =
  'inline-flex items-center gap-1 rounded border border-border bg-surface px-1.5 py-0.5 text-[11px] font-medium text-text-secondary';

function useCrumbs(appName?: string, nodeName?: string): Crumb[] {
  const location = useLocation();
  const params = useParams();
  const segments = location.pathname.split('/').filter(Boolean);
  const [section, second, third] = segments;

  if (section === 'apps') {
    const crumbs: Crumb[] = [{ label: 'Apps', to: '/apps' }];
    if (!second) {
      return crumbs;
    }
    if (second === 'new') {
      return [...crumbs, { label: 'New app' }];
    }

    const appPath = `/apps/${params.slug}`;
    crumbs.push({ label: appName ?? params.slug!, to: appPath });

    if (third === 'crons' && segments.length > 3) {
      crumbs.push({ label: 'Crons', to: `${appPath}/crons` });
      crumbs.push({
        label: segments.at(-1) === 'new' ? 'New job' : 'Job',
      });
    }
    if (third === 'deployments' && segments.length > 3) {
      crumbs.push({ label: 'Deployments', to: `${appPath}/deployments` });
      crumbs.push({ label: 'Deployment' });
    }
    if (third === 'services' && segments.length > 3) {
      crumbs.push({ label: 'Services', to: `${appPath}/services` });
      crumbs.push({ label: 'Service' });
    }
    return crumbs;
  }

  if (section === 'nodes') {
    const crumbs: Crumb[] = [{ label: 'Nodes', to: '/nodes' }];
    if (!second) {
      return crumbs;
    }
    if (second === 'new') {
      return [...crumbs, { label: 'Add node' }];
    }
    return [
      ...crumbs,
      { label: nodeName ?? params.id!, to: `/nodes/${params.id}` },
    ];
  }

  if (section === 'alerts') {
    const crumbs: Crumb[] = [{ label: 'Alerts', to: '/alerts' }];
    if (second === 'channels') {
      crumbs.push({ label: 'Channels', to: '/alerts/channels' });
    }
    if (second === 'rules') {
      crumbs.push({ label: 'Rules', to: '/alerts/rules' });
    }
    if (second === 'incidents') {
      crumbs.push({ label: 'Alert' });
    }
    if (third === 'new') {
      crumbs.push({ label: second === 'rules' ? 'New rule' : 'New channel' });
    }
    if (third === 'edit' || segments.at(-1) === 'edit') {
      crumbs.push({ label: 'Edit' });
    }
    return crumbs;
  }

  if (section === 'users') {
    const crumbs: Crumb[] = [{ label: 'Users', to: '/users' }];
    if (second === 'new') {
      crumbs.push({ label: 'Add user' });
    }
    if (second && second !== 'new') {
      crumbs.push({ label: 'Edit user' });
    }
    return crumbs;
  }

  if (section === 'settings') {
    return [{ label: 'Settings', to: '/settings' }];
  }

  return [];
}

type CloneUrlProps = {
  app: App;
};

function CloneUrl(props: CloneUrlProps) {
  const { app } = props;
  const [protocol, setProtocol] = useState<'https' | 'ssh'>('https');
  const [copied, setCopied] = useState(false);

  const { httpsUrl, sshUrl } = useMemo(() => {
    if (typeof window === 'undefined') {
      return {
        httpsUrl: `/git/${app.slug}`,
        sshUrl: `slasha@localhost:${app.slug}.git`,
      };
    }
    return {
      httpsUrl: `${window.location.origin}/git/${app.slug}`,
      sshUrl: `slasha@${window.location.hostname}:${app.slug}.git`,
    };
  }, [app.slug]);

  const url = protocol === 'https' ? httpsUrl : sshUrl;

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => {
        setCopied(false);
      }, 1500);
    } catch {}
  };

  return (
    <div className="flex items-center rounded border border-border bg-surface">
      {(['https', 'ssh'] as const).map((item) => (
        <button
          key={item}
          onClick={() => {
            setProtocol(item);
          }}
          className={cn(
            'h-6 px-2 text-[11px] font-medium uppercase transition-colors',
            item === 'ssh' && 'border-l border-border',
            protocol === item
              ? 'bg-white/[0.06] text-text'
              : 'text-text-tertiary hover:text-text'
          )}
        >
          {item}
        </button>
      ))}
      <code className="max-w-[280px] truncate border-l border-border px-2 font-mono text-[11px] text-text-secondary">
        {url}
      </code>
      <button
        onClick={handleCopy}
        aria-label="Copy clone URL"
        className="flex h-6 w-7 items-center justify-center border-l border-border text-text-tertiary transition-colors hover:text-text"
      >
        {copied ? (
          <Check className="size-3 text-emerald-400" />
        ) : (
          <Copy className="size-3" />
        )}
      </button>
    </div>
  );
}

export function TopBar() {
  const params = useParams();
  const location = useLocation();
  const isAppRoute = location.pathname.startsWith('/apps/') && !!params.slug;
  const isNodeRoute = location.pathname.startsWith('/nodes/') && !!params.id;

  const { data: appData } = useQuery({
    ...getAppOptions(params.slug!),
    enabled: isAppRoute,
  });
  const { data: nodeData } = useQuery({
    ...getNodeOptions(params.id!),
    enabled: isNodeRoute,
  });

  const app = isAppRoute ? appData?.app : undefined;
  const node = isNodeRoute ? nodeData?.node : undefined;
  const crumbs = useCrumbs(app?.name, node?.name);
  // Branch, runtime and clone URL describe the app. On a nested entity (a service,
  // a deployment, a job) that entity carries its own status, so showing the app's
  // too reads as if it were the entity's.
  const isAppItself = crumbs.length <= 2;

  return (
    <header className="flex h-12 shrink-0 items-center justify-between gap-4 border-b border-border px-8">
      <HStack space={2} className="min-w-0">
        <nav
          aria-label="Breadcrumb"
          className="flex min-w-0 items-center gap-1"
        >
          {crumbs.map((crumb, index) => {
            const isLast = index === crumbs.length - 1;
            return (
              <HStack key={crumb.label + index} space={1} className="min-w-0">
                {index > 0 ? (
                  <ChevronRight className="size-3 shrink-0 text-text-tertiary/60" />
                ) : null}
                {crumb.to && !isLast ? (
                  <Link
                    to={crumb.to}
                    className="truncate text-[13px] text-text-tertiary !no-underline transition-colors hover:!text-text"
                  >
                    {crumb.label}
                  </Link>
                ) : (
                  <span className="truncate text-[13px] font-medium text-text">
                    {crumb.label}
                  </span>
                )}
              </HStack>
            );
          })}
        </nav>

        {app && isAppItself ? (
          <HStack space={2} className="shrink-0 pl-1">
            <span className={chipClasses}>
              <GitBranch className="size-3" />
              {app.default_branch}
            </span>
            <AppRuntimeBadge
              status={appData?.runtime_status as AppRuntimeStatus}
            />
            {appData?.runtime_status === 'running' && appData.url ? (
              <a
                href={appData.url}
                target="_blank"
                rel="noreferrer"
                className={cn(
                  chipClasses,
                  '!no-underline transition-colors hover:bg-white/5 hover:text-text'
                )}
              >
                Visit
                <ArrowUpRight className="size-3" />
              </a>
            ) : null}
          </HStack>
        ) : null}

        {node ? (
          <HStack space={2} className="shrink-0 pl-1">
            <span className={chipClasses}>
              {node.id === 'local' ? (
                'Local'
              ) : (
                <span className="font-mono">
                  {node.user}
                  <span className="font-sans text-text-tertiary">@</span>
                  {node.host}
                </span>
              )}
            </span>
            <NodeStatusBadge
              status={node.status}
              connectionStatus={node.connection_status}
            />
          </HStack>
        ) : null}
      </HStack>

      {app && isAppItself && app.source === 'local' ? (
        <CloneUrl app={app} />
      ) : null}
    </header>
  );
}
