import { useMemo } from 'react';
import { Folder } from 'lucide-react';
import { getFileIcon } from '~/utils/file-icon';
import type { FileTreeNode } from '~/queries/files';
import { PathBreadcrumb } from './path-breadcrumb';

type FolderViewerProps = {
  node: FileTreeNode;
  onSelect: (path: string) => void;
}

export function FolderViewer(props: FolderViewerProps) {
  const { node, onSelect } = props;

  const children = useMemo(() => {
    const list = [...(node.children ?? [])];
    list.sort((a, b) => {
      if (a.node_type !== b.node_type) {
        return a.node_type === 'directory' ? -1 : 1;
      }
      return a.name.localeCompare(b.name);
    });
    return list;
  }, [node.children]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between gap-3 border-b border-border px-3 py-2">
        <PathBreadcrumb path={node.path} isDirectory onNavigate={onSelect} />
        <span className="shrink-0 text-[11px] text-text-tertiary">
          {children.length} {children.length === 1 ? 'item' : 'items'}
        </span>
      </div>

      <div className="flex-1 overflow-auto">
        {children.length === 0 ? (
          <div className="flex h-full items-center justify-center text-[12px] text-text-tertiary">
            Empty folder
          </div>
        ) : (
          <ul className="divide-y divide-border">
            {children.map((child) => {
              const isDir = child.node_type === 'directory';
              const Icon = isDir ? Folder : getFileIcon(child.name);
              return (
                <li key={child.path}>
                  <button
                    type="button"
                    onClick={() => {
                      onSelect(child.path);
                    }}
                    className="group flex w-full items-center gap-2 px-4 py-2 text-left text-[13px] text-text-secondary transition-colors hover:bg-white/[0.04] hover:text-text"
                  >
                    {Icon ? (
                      <Icon className="size-3.5 shrink-0 text-text-tertiary" />
                    ) : null}
                    <span className="truncate">{child.name}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
