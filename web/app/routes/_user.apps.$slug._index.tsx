import { useCallback, useMemo, useState } from 'react';
import { useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { FileText, GitBranch } from 'lucide-react';
import { findNodeByPath, getFileTreeOptions } from '~/queries/files';
import type { FileTreeNode } from '~/queries/files';
import { FileTree } from '~/components/apps/file-tree';
import { CodeViewer } from '~/components/apps/code-viewer';
import { FolderViewer } from '~/components/apps/folder-viewer';
import { EmptyPage } from '~/components/global/empty-page';
import { queryClient } from '~/utils/query-client';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await queryClient.ensureQueryData(getFileTreeOptions(params.slug));
}

export default function AppFilesPage() {
  const { slug } = useParams();
  const { data: treeData } = useSuspenseQuery(getFileTreeOptions(slug!));

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

  const tree: FileTreeNode[] = treeData.tree ?? [];

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

  const hasCommits = treeData.has_commits ?? false;

  if (!hasCommits) {
    return (
      <div className="flex h-full min-h-0 flex-1 flex-col">
        <EmptyPage
          className="flex-1"
          icon={GitBranch}
          title="No commits yet"
          subtitle="Push code to this repository to see the file tree."
        />
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col">
      <div className="flex min-h-0 flex-1 overflow-hidden">
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
    </div>
  );
}
