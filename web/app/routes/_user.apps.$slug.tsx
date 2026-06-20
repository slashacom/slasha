import { Suspense, useMemo, useState } from 'react';
import { NavLink, Outlet, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Check, Copy, Folder, GitBranch } from 'lucide-react';
import { getAppOptions } from '~/queries/apps';
import type { App } from '~/models/app';
import { HStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import { queryClient } from '~/utils/query-client';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await queryClient.ensureQueryData(getAppOptions(params.slug));
}

export function meta() {
  return [{ title: 'App · slasha' }];
}

type Protocol = 'https' | 'ssh';

type AppToolbarProps = {
  app: App;
};

function AppToolbar(props: AppToolbarProps) {
  const { app } = props;
  const [protocol, setProtocol] = useState<Protocol>('https');
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
    <div className="flex shrink-0 items-center justify-between gap-4 border-b border-border px-8 py-3">
      <div className="flex min-w-0 items-center gap-3">
        <Folder className="size-4 shrink-0 text-text-tertiary" />
        <span className="truncate text-[13px] font-medium text-text">
          {app.name}
        </span>
        <span className="font-mono text-[12px] text-text-tertiary">
          {app.slug}
        </span>
        <span className="inline-flex items-center gap-1 rounded border border-border bg-surface px-1.5 py-0.5 text-[11px] font-medium text-text-secondary">
          <GitBranch className="size-3" />
          {app.default_branch}
        </span>
      </div>

      <div className="flex items-center rounded border border-border bg-surface">
        <button
          onClick={() => {
            setProtocol('https');
          }}
          className={cn(
            'h-7 px-2.5 text-[11px] font-medium transition-colors',
            protocol === 'https'
              ? 'bg-white/[0.06] text-text'
              : 'text-text-tertiary hover:text-text'
          )}
        >
          HTTPS
        </button>
        <button
          onClick={() => {
            setProtocol('ssh');
          }}
          className={cn(
            'h-7 border-l border-border px-2.5 text-[11px] font-medium transition-colors',
            protocol === 'ssh'
              ? 'bg-white/[0.06] text-text'
              : 'text-text-tertiary hover:text-text'
          )}
        >
          SSH
        </button>
        <div className="h-7 w-px bg-border" />
        <code className="max-w-[320px] truncate px-2.5 font-mono text-[11px] text-text-secondary">
          {url}
        </code>
        <button
          onClick={handleCopy}
          aria-label="Copy clone URL"
          className="flex h-7 w-8 items-center justify-center border-l border-border text-text-tertiary transition-colors hover:text-text"
        >
          {copied ? (
            <Check className="size-3.5 text-emerald-400" />
          ) : (
            <Copy className="size-3.5" />
          )}
        </button>
      </div>
    </div>
  );
}

type TabLinkProps = {
  to: string;
  end?: boolean;
  children: React.ReactNode;
};

function TabLink(props: TabLinkProps) {
  const { to, end, children } = props;
  return (
    <NavLink
      to={to}
      end={end}
      className={({ isActive }) =>
        cn(
          'flex h-10 items-center text-[13px] font-medium transition-colors border-b-2 -mb-[2px]',
          isActive
            ? 'border-white text-text'
            : 'border-transparent text-text-tertiary hover:text-text-secondary'
        )
      }
    >
      {children}
    </NavLink>
  );
}

export default function AppLayout() {
  const { slug } = useParams();
  const { data } = useSuspenseQuery(getAppOptions(slug!));
  const app = data.app;

  if (!app) {
    return (
      <div className="p-8">
        <h3 className="font-semibold text-text">App not found</h3>
        <p className="mt-2 text-sm text-text-secondary">
          The application you're looking for doesn't exist.
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-1 flex-col min-h-0">
      <AppToolbar app={app} />

      <div className="flex shrink-0 border-b border-border bg-surface/30 px-8">
        <HStack space={6}>
          <TabLink to={`/apps/${slug}`} end>
            Files
          </TabLink>
          <TabLink to={`/apps/${slug}/deployments`}>Deployments</TabLink>
          <TabLink to={`/apps/${slug}/services`}>Services</TabLink>
          <TabLink to={`/apps/${slug}/metrics`}>Metrics</TabLink>
          <TabLink to={`/apps/${slug}/settings`}>Settings</TabLink>
        </HStack>
      </div>

      <Suspense fallback={null}>
        <Outlet />
      </Suspense>
    </div>
  );
}
