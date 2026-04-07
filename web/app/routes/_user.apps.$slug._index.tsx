import { useCallback, useEffect, useState } from 'react';
import { useParams } from 'react-router';
import { useQuery } from '@tanstack/react-query';
import { FileText, GitBranch } from 'lucide-react';
import { getAppOptions } from '~/queries/apps';
import { getFileTreeOptions } from '~/queries/files';
import type { FileTreeNode } from '~/queries/files';
import { PageContainer } from '~/components/interface/page-container';
import { VStack, HStack } from '~/components/interface/stacks';
import { Skeleton } from '~/components/interface/skeleton';
import { FileTree } from '~/components/apps/file-tree';
import { CodeViewer } from '~/components/apps/code-viewer';
import { queryClient } from '~/utils/query-client';

export async function clientLoader({ params }: { params: { slug: string } }) {
  await queryClient.ensureQueryData(getAppOptions(params.slug));
  await queryClient.ensureQueryData(getFileTreeOptions(params.slug));
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
      <PageContainer variant="center" className="py-10">
        <VStack space={3}>
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-96 w-full" />
        </VStack>
      </PageContainer>
    );
  }

  const app = appData?.app;
  if (!app) {
    return (
      <PageContainer variant="center" className="py-10">
        <h1 className="text-xl font-bold">App not found</h1>
      </PageContainer>
    );
  }

  const hasCommits = treeData?.has_commits ?? false;
  const tree: FileTreeNode[] = treeData?.tree ?? [];

  return (
    <PageContainer variant="center" className="px-6 py-10">
      <VStack space={6}>
        <HStack justifyContent="between" alignItems="center">
          <VStack space={1}>
            <h1 className="text-4xl font-bold tracking-tight text-black">
              {app.name}
            </h1>
            <p className="font-mono text-sm text-neutral-500">{app.slug}</p>
          </VStack>
          <div className="rounded-full border border-neutral-200 bg-neutral-50 px-3 py-1 text-xs font-medium text-neutral-600">
            {app.default_branch}
          </div>
        </HStack>

        <div className="h-px w-full bg-neutral-100" />

        {!hasCommits ? (
          <div className="flex h-full flex-col items-center justify-center gap-4 py-20">
            <div className="rounded-full bg-neutral-50 p-4">
              <GitBranch className="size-8 text-neutral-300" />
            </div>
            <VStack space={1} alignItems="center">
              <p className="text-sm font-medium text-neutral-700">
                No commits yet
              </p>
              <p className="text-sm text-neutral-400">
                Push code to this repository to see the file tree.
              </p>
            </VStack>
          </div>
        ) : (
          <div className="flex h-[calc(100vh-220px)] overflow-hidden rounded-lg border border-neutral-200">
            <FileTree
              tree={tree}
              selectedPath={selectedPath}
              onSelect={handleSelect}
              expandedPaths={expandedPaths}
              onToggle={handleToggle}
            />
            <div className="flex-1 overflow-hidden bg-white">
              {selectedPath ? (
                <CodeViewer slug={slug!} filePath={selectedPath} />
              ) : (
                <div className="flex h-full flex-col items-center justify-center gap-3 text-neutral-400">
                  <FileText className="size-8" />
                  <p className="text-sm">Select a file to view its contents</p>
                </div>
              )}
            </div>
          </div>
        )}
      </VStack>
    </PageContainer>
  );
}
