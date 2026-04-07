import { useCallback, useState } from 'react';
import { useParams } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import { FileText, GitBranch } from 'lucide-react';
import { getAppOptions } from '~/queries/apps';
import { getFileTreeOptions } from '~/queries/files';
import type { FileTreeNode } from '~/queries/files';
import { Skeleton } from '~/components/interface/skeleton';
import { FileTree } from '~/components/apps/file-tree';
import { CodeViewer } from '~/components/apps/code-viewer';
import { queryClient } from '~/utils/query-client';

export async function clientLoader({ params }: { params: { slug: string } }) {
  await queryClient.ensureQueryData(getAppOptions(params.slug));
  await queryClient.ensureQueryData(getFileTreeOptions(params.slug));
}

export function meta() {
  return [{ title: 'App · slasha' }];
}

export default function AppIndexPage() {
  const { slug } = useParams();
  const { data: appData, isLoading: appLoading } = useQuery(
    getAppOptions(slug!)
  );
  const { data: treeData, isLoading: treeLoading } = useQuery(
    getFileTreeOptions(slug!)
  );

  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());

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

  const handleSelect = useCallback((path: string) => {
    setSelectedPath(path);
  }, []);

  if (appLoading || treeLoading) {
    return (
      <div className="space-y-3">
        <Skeleton className="h-6 w-48 bg-surface" />
        <Skeleton className="h-4 w-32 bg-surface" />
        <Skeleton className="mt-6 h-96 w-full bg-surface" />
      </div>
    );
  }

  const app = appData?.app;
  if (!app) {
    return (
      <div>
        <h3 className="font-semibold text-text">App not found</h3>
        <p className="mt-2 text-sm text-text-secondary">
          The application you're looking for doesn't exist.
        </p>
      </div>
    );
  }

  const hasCommits = treeData?.has_commits ?? false;
  const tree: FileTreeNode[] = treeData?.tree ?? [];

  return (
    <div>
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold text-text">{app.name}</h3>
          <p className="mt-2 font-mono text-sm text-text-tertiary">
            {app.slug}
          </p>
        </div>
        <span className="rounded border border-border bg-surface px-2 py-0.5 text-[11px] font-medium text-text-secondary">
          {app.default_branch}
        </span>
      </div>

      <div className="mt-6">
        {!hasCommits ? (
          <div className="flex flex-col items-center justify-center gap-3 rounded-lg border border-border bg-surface py-16">
            <div className="rounded-full border border-border p-3">
              <GitBranch className="size-5 text-text-tertiary" />
            </div>
            <p className="text-sm font-medium text-text">No commits yet</p>
            <p className="text-xs text-text-tertiary">
              Push code to this repository to see the file tree.
            </p>
          </div>
        ) : (
          <div className="flex h-[calc(100vh-200px)] overflow-hidden rounded-lg border border-border bg-surface">
            <FileTree
              tree={tree}
              selectedPath={selectedPath}
              onSelect={handleSelect}
              expandedPaths={expandedPaths}
              onToggle={handleToggle}
            />
            <div className="flex-1 overflow-hidden">
              {selectedPath ? (
                <CodeViewer slug={slug!} filePath={selectedPath} />
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
    </div>
  );
}
