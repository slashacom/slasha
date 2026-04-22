import { useCallback, useMemo, useState } from 'react';
import { useParams } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import {
  Check,
  Copy,
  FileText,
  Folder,
  GitBranch,
  Loader2,
  Settings,
  Trash,
} from 'lucide-react';
import { useNavigate } from 'react-router';
import { getAppOptions, useDeleteApp } from '~/queries/apps';
import { findNodeByPath, getFileTreeOptions } from '~/queries/files';
import type { FileTreeNode } from '~/queries/files';
import type { App } from '~/models/app';
import { Skeleton } from '~/components/interface/skeleton';
import { FileTree } from '~/components/apps/file-tree';
import { CodeViewer } from '~/components/apps/code-viewer';
import { FolderViewer } from '~/components/apps/folder-viewer';
import { DeploymentsView } from '~/components/apps/deployments';
import { ServicesView } from '~/components/apps/services';
import { AppEnvEditor } from '~/components/apps/env-editor';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { cn } from '~/utils/classname';
import { queryClient } from '~/utils/query-client';
import { getDeploymentsOptions } from '~/queries/deployments';
import { getAppServicesOptions } from '~/queries/services';
import { HStack } from '~/components/interface/stacks';

export async function clientLoader({ params }: { params: { slug: string } }) {
  await Promise.all([
    queryClient.ensureQueryData(getAppOptions(params.slug)),
    queryClient.ensureQueryData(getFileTreeOptions(params.slug)),
    queryClient.ensureQueryData(getDeploymentsOptions(params.slug)),
    queryClient.ensureQueryData(getAppServicesOptions(params.slug)),
  ]);
}

export function meta() {
  return [{ title: 'App · slasha' }];
}

type Protocol = 'https' | 'ssh';

function AppToolbar(props: { app: App }) {
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

export default function AppIndexPage() {
  const { slug } = useParams();
  const { data: appData, isLoading: appLoading } = useQuery(
    getAppOptions(slug!)
  );
  const { data: treeData, isLoading: treeLoading } = useQuery(
    getFileTreeOptions(slug!)
  );

  const navigate = useNavigate();
  const deleteApp = useDeleteApp();
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [activeTab, setActiveTab] = useState<
    'files' | 'deployments' | 'services' | 'settings'
  >('files');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  const handleToggle = useCallback((path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const tree: FileTreeNode[] = treeData?.tree ?? [];

  const handleSelect = useCallback(
    (path: string) => {
      setSelectedPath(path);
      const node = findNodeByPath(tree, path);
      if (node?.node_type === 'directory') {
        setExpandedPaths((prev) => {
          if (prev.has(path)) {
            return prev;
          }
          const next = new Set(prev);
          next.add(path);
          return next;
        });
      }
    },
    [tree]
  );

  const selectedNode = useMemo(() => {
    if (!selectedPath) {
      return null;
    }
    return findNodeByPath(tree, selectedPath);
  }, [tree, selectedPath]);

  if (appLoading || treeLoading) {
    return (
      <div className="flex flex-1 flex-col">
        <div className="flex shrink-0 items-center justify-between gap-4 border-b border-border px-8 py-3">
          <div className="flex items-center gap-3">
            <Skeleton className="size-4 bg-surface" />
            <Skeleton className="h-4 w-32 bg-surface" />
            <Skeleton className="h-4 w-20 bg-surface" />
          </div>
          <Skeleton className="h-7 w-64 bg-surface" />
        </div>
        <div className="flex flex-1 min-h-0">
          <div className="w-64 shrink-0 border-r border-border p-3">
            <Skeleton className="mb-3 h-7 w-full bg-surface" />
            <div className="space-y-2">
              {[...Array(8)].map((_, i) => (
                <Skeleton key={i} className="h-5 w-full bg-surface" />
              ))}
            </div>
          </div>
          <div className="flex-1 p-6">
            <Skeleton className="h-full w-full bg-surface" />
          </div>
        </div>
      </div>
    );
  }

  const app = appData?.app;
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

  const hasCommits = treeData?.has_commits ?? false;

  return (
    <div className="flex flex-1 flex-col min-h-0">
      <AppToolbar app={app} />

      <div className="flex shrink-0 border-b border-border bg-surface/30 px-8">
        <HStack space={6}>
          <button
            onClick={() => setActiveTab('files')}
            className={cn(
              'h-10 text-[13px] font-medium transition-colors border-b-2 -mb-[2px]',
              activeTab === 'files'
                ? 'border-white text-text'
                : 'border-transparent text-text-tertiary hover:text-text-secondary'
            )}
          >
            Files
          </button>
            <button
            onClick={() => setActiveTab('deployments')}
            className={cn(
              'h-10 text-[13px] font-medium transition-colors border-b-2 -mb-[2px]',
              activeTab === 'deployments'
                ? 'border-white text-text'
                : 'border-transparent text-text-tertiary hover:text-text-secondary'
            )}
          >
            Deployments
          </button>
          <button
            onClick={() => setActiveTab('services')}
            className={cn(
              'h-10 text-[13px] font-medium transition-colors border-b-2 -mb-[2px]',
              activeTab === 'services'
                ? 'border-white text-text'
                : 'border-transparent text-text-tertiary hover:text-text-secondary'
            )}
          >
            Services
          </button>
          <button
            onClick={() => setActiveTab('settings')}
            className={cn(
              'h-10 text-[13px] font-medium transition-colors border-b-2 -mb-[2px]',
              activeTab === 'settings'
                ? 'border-white text-text'
                : 'border-transparent text-text-tertiary hover:text-text-secondary'
            )}
          >
            Settings
          </button>
        </HStack>
      </div>

      {activeTab === 'settings' ? (
        <div className="flex-1 overflow-y-auto p-8">
          <div className="max-w-3xl mb-12">
            <AppEnvEditor appSlug={slug!} />
          </div>
          <div className="max-w-2xl">
            <h3 className="text-[14px] font-semibold text-text">Danger Zone</h3>
            <p className="mt-1 text-[13px] text-text-tertiary">
              Destructive actions for your application.
            </p>

            <div className="mt-6 rounded-lg border border-red-500/20 bg-red-500/5 p-6">
              <div className="flex items-start justify-between gap-6">
                <div>
                  <h4 className="text-[13px] font-medium text-red-500">
                    Delete this application
                  </h4>
                  <p className="mt-1 text-[12px] text-red-500/70">
                    Once you delete an application, there is no going back.
                    Please be certain.
                  </p>
                </div>
                <button
                  onClick={() => setShowDeleteConfirm(true)}
                  className="shrink-0 rounded-md bg-red-600 px-3 py-1.5 text-[12px] font-medium text-white transition-colors hover:bg-red-500"
                >
                  Delete App
                </button>
              </div>
            </div>
          </div>

          <ConfirmationDialog
            open={showDeleteConfirm}
            onOpenChange={setShowDeleteConfirm}
            title="Delete Application"
            description={`Are you sure you want to delete ${app.name}? This action cannot be undone and will permanently delete all associated data.`}
            confirmLabel="Delete Application"
            onConfirm={() => {
              deleteApp.mutate(app.slug, {
                onSuccess: () => {
                  navigate('/apps');
                },
              });
            }}
          />
        </div>
      ) : activeTab === 'deployments' ? (
        <DeploymentsView appSlug={slug!} />
      ) : activeTab === 'services' ? (
        <ServicesView appSlug={slug!} />
      ) : !hasCommits ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3">
          <div className="rounded-full border border-border p-3">
            <GitBranch className="size-5 text-text-tertiary" />
          </div>
          <p className="text-sm font-medium text-text">No commits yet</p>
          <p className="text-xs text-text-tertiary">
            Push code to this repository to see the file tree.
          </p>
        </div>
      ) : (
        <div className="flex flex-1 min-h-0 overflow-hidden">
          <FileTree
            tree={tree}
            selectedPath={selectedPath}
            onSelect={handleSelect}
            expandedPaths={expandedPaths}
            onToggle={handleToggle}
          />
          <div className="flex-1 min-w-0 overflow-hidden">
            {selectedNode ? (
              selectedNode.node_type === 'directory' ? (
                <FolderViewer node={selectedNode} onSelect={handleSelect} />
              ) : (
                <CodeViewer
                  slug={slug!}
                  filePath={selectedNode.path}
                  onNavigate={handleSelect}
                />
              )
            ) : (
              <div className="flex h-full flex-col items-center justify-center gap-3 text-text-tertiary">
                <FileText className="size-6" />
                <p className="text-xs">Select a file to view its contents</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
