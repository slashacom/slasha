import { queryOptions } from '@tanstack/react-query';
import { httpGet } from '~/utils/http';

export type FileTreeNode = {
  name: string;
  path: string;
  node_type: 'file' | 'directory';
  size?: number;
  children?: FileTreeNode[];
}

export type FileTreeResponse = {
  tree: FileTreeNode[];
  has_commits: boolean;
}

export function findNodeByPath(
  tree: FileTreeNode[],
  path: string
): FileTreeNode | null {
  for (const node of tree) {
    if (node.path === path) {
      return node;
    }
    if (node.children) {
      const found = findNodeByPath(node.children, path);
      if (found) {
        return found;
      }
    }
  }
  return null;
}

export type FileContentResponse = {
  path: string;
  name: string;
  size: number;
  is_binary: boolean;
  is_truncated: boolean;
  content: string | null;
}

export function getFileTreeOptions(slug: string) {
  return queryOptions({
    queryKey: ['apps', slug, 'files'],
    queryFn: () => httpGet<FileTreeResponse>(`apps/${slug}/files`),
  });
}

export function getFileContentOptions(slug: string, path: string) {
  return queryOptions({
    queryKey: ['apps', slug, 'files', path],
    queryFn: () => httpGet<FileContentResponse>(`apps/${slug}/files/${path}`),
    enabled: !!path,
  });
}
