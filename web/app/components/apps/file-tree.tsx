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
          'flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-left text-[13px] transition-colors',
          'hover:bg-neutral-100',
          isSelected && !isDir && 'bg-neutral-100 font-medium text-black',
          !isSelected && 'text-neutral-700'
        )}
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
      >
        {isDir ? (
          <>
            {isExpanded ? (
              <ChevronDown className="size-3.5 shrink-0 text-neutral-400" />
            ) : (
              <ChevronRight className="size-3.5 shrink-0 text-neutral-400" />
            )}
            {isExpanded ? (
              <FolderOpen className="size-3.5 shrink-0 text-neutral-500" />
            ) : (
              <Folder className="size-3.5 shrink-0 text-neutral-500" />
            )}
          </>
        ) : (
          <>
            <span className="size-3.5 shrink-0" />
            <FileText className="size-3.5 shrink-0 text-neutral-400" />
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
    <div className="w-72 shrink-0 overflow-y-auto border-r border-neutral-100 bg-neutral-50/50 py-2">
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
