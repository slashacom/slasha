import {
  ChevronRight,
  ChevronDown,
  FileText,
  Folder,
  FolderOpen,
} from 'lucide-react';
import { cn } from '~/utils/classname';
import type { FileTreeNode } from '~/queries/files';

interface TreeNodeProps {
  node: FileTreeNode;
  depth: number;
  selectedPath: string | null;
  onSelect: (path: string) => void;
  expandedPaths: Set<string>;
  onToggle: (path: string) => void;
}

function TreeNode({
  node,
  depth,
  selectedPath,
  onSelect,
  expandedPaths,
  onToggle,
}: TreeNodeProps) {
  const isDir = node.node_type === 'directory';
  const isExpanded = expandedPaths.has(node.path);
  const isSelected = selectedPath === node.path;

  const handleClick = () => {
    if (isDir) {
      onToggle(node.path);
    } else {
      onSelect(node.path);
    }
  };

  return (
    <div>
      <button
        onClick={handleClick}
        className={cn(
          'flex w-full items-center gap-1.5 px-2 py-1 text-left text-[13px] transition-colors',
          'hover:bg-white/[0.04]',
          isSelected && !isDir && 'bg-white/[0.06] text-text',
          !isSelected && 'text-text-secondary'
        )}
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
      >
        {isDir ? (
          <>
            {isExpanded ? (
              <ChevronDown className="size-3.5 shrink-0 text-text-tertiary" />
            ) : (
              <ChevronRight className="size-3.5 shrink-0 text-text-tertiary" />
            )}
            {isExpanded ? (
              <FolderOpen className="size-3.5 shrink-0 text-text-tertiary" />
            ) : (
              <Folder className="size-3.5 shrink-0 text-text-tertiary" />
            )}
          </>
        ) : (
          <>
            <span className="size-3.5 shrink-0" />
            <FileText className="size-3.5 shrink-0 text-text-tertiary" />
          </>
        )}
        <span className="truncate">{node.name}</span>
      </button>

      {isDir && isExpanded && node.children && (
        <div>
          {node.children.map((child) => (
            <TreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              selectedPath={selectedPath}
              onSelect={onSelect}
              expandedPaths={expandedPaths}
              onToggle={onToggle}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface FileTreeProps {
  tree: FileTreeNode[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
  expandedPaths: Set<string>;
  onToggle: (path: string) => void;
}

export function FileTree({
  tree,
  selectedPath,
  onSelect,
  expandedPaths,
  onToggle,
}: FileTreeProps) {
  return (
    <div className="w-56 shrink-0 overflow-y-auto border-r border-border bg-bg/40 py-2">
      {tree.map((node) => (
        <TreeNode
          key={node.path}
          node={node}
          depth={0}
          selectedPath={selectedPath}
          onSelect={onSelect}
          expandedPaths={expandedPaths}
          onToggle={onToggle}
        />
      ))}
    </div>
  );
}
