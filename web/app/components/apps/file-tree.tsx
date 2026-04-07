import { useMemo, useState } from 'react';
import { ChevronRight, Folder, FolderOpen, Search, X } from 'lucide-react';
import { cn } from '~/utils/classname';
import { getFileIcon } from '~/utils/file-icon';
import type { FileTreeNode } from '~/queries/files';

interface TreeNodeProps {
  node: FileTreeNode;
  depth: number;
  selectedPath: string | null;
  onSelect: (path: string) => void;
  expandedPaths: Set<string>;
  onToggle: (path: string) => void;
}

function TreeNode(props: TreeNodeProps) {
  const { node, depth, selectedPath, onSelect, expandedPaths, onToggle } =
    props;
  const isDir = node.node_type === 'directory';
  const isExpanded = expandedPaths.has(node.path);
  const isSelected = selectedPath === node.path;

  const handleSelect = () => {
    onSelect(node.path);
  };

  const handleToggle = () => {
    onToggle(node.path);
  };

  const FileIcon = !isDir ? getFileIcon(node.name) : null;

  return (
    <div>
      <div
        className={cn(
          'group relative flex w-full items-center py-[5px] pr-2 text-[13px] transition-colors',
          'hover:bg-white/[0.04]',
          isSelected && 'bg-white/[0.07] text-text',
          isSelected &&
            'after:pointer-events-none after:absolute after:inset-y-0 after:left-0 after:w-[2px] after:bg-text',
          !isSelected && 'text-text-secondary'
        )}
        style={{ paddingLeft: `${depth * 12 + 10}px` }}
      >
        <button
          type="button"
          onClick={handleSelect}
          aria-label={`Open ${node.name}`}
          className="absolute inset-0"
        />

        {isDir ? (
          <button
            type="button"
            onClick={handleToggle}
            aria-label={isExpanded ? 'Collapse folder' : 'Expand folder'}
            className="relative z-10 flex size-4 shrink-0 items-center justify-center text-text-tertiary hover:text-text before:absolute before:-inset-1.5 before:content-['']"
          >
            <ChevronRight
              className={cn(
                'size-3 transition-transform',
                isExpanded && 'rotate-90'
              )}
            />
          </button>
        ) : (
          <span className="size-4 shrink-0" />
        )}
        <div className="pointer-events-none relative flex min-w-0 flex-1 items-center gap-1.5 pl-1 group-hover:text-text">
          {isDir ? (
            isExpanded ? (
              <FolderOpen className="size-3.5 shrink-0 text-text-secondary" />
            ) : (
              <Folder className="size-3.5 shrink-0 text-text-tertiary" />
            )
          ) : FileIcon ? (
            <FileIcon className="size-3.5 shrink-0 text-text-tertiary" />
          ) : null}
          <span className="truncate">{node.name}</span>
        </div>
      </div>

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

function filterTree(
  nodes: FileTreeNode[],
  query: string
): { nodes: FileTreeNode[]; matchedDirs: string[] } {
  const lower = query.toLowerCase();
  const matchedDirs: string[] = [];

  const walk = (list: FileTreeNode[]): FileTreeNode[] => {
    const out: FileTreeNode[] = [];
    for (const node of list) {
      if (node.node_type === 'directory') {
        const childMatches = node.children ? walk(node.children) : [];
        const selfMatches = node.name.toLowerCase().includes(lower);
        if (childMatches.length > 0 || selfMatches) {
          matchedDirs.push(node.path);
          out.push({
            ...node,
            children: childMatches.length > 0 ? childMatches : node.children,
          });
        }
        continue;
      }
      if (node.name.toLowerCase().includes(lower)) {
        out.push(node);
      }
    }
    return out;
  };

  return { nodes: walk(nodes), matchedDirs };
}

interface FileTreeProps {
  tree: FileTreeNode[];
  selectedPath: string | null;
  onSelect: (path: string) => void;
  expandedPaths: Set<string>;
  onToggle: (path: string) => void;
}

export function FileTree(props: FileTreeProps) {
  const { tree, selectedPath, onSelect, expandedPaths, onToggle } = props;
  const [query, setQuery] = useState('');

  const { displayTree, searchExpanded } = useMemo(() => {
    if (!query.trim()) {
      return { displayTree: tree, searchExpanded: null as Set<string> | null };
    }
    const { nodes, matchedDirs } = filterTree(tree, query.trim());
    return { displayTree: nodes, searchExpanded: new Set(matchedDirs) };
  }, [tree, query]);

  const effectiveExpanded = searchExpanded ?? expandedPaths;

  return (
    <div className="flex w-64 shrink-0 flex-col border-r border-border">
      <div className="px-3 pt-3 pb-2">
        <div className="relative">
          <Search className="pointer-events-none absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-text-tertiary" />
          <input
            type="text"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
            }}
            placeholder="Search files…"
            className="h-7 w-full rounded bg-white/[0.04] pl-7 pr-7 text-[12px] text-text placeholder:text-text-tertiary focus:bg-white/[0.06] focus:outline-none"
          />
          {query && (
            <button
              onClick={() => {
                setQuery('');
              }}
              className="absolute right-1.5 top-1/2 -translate-y-1/2 rounded p-0.5 text-text-tertiary hover:bg-white/[0.06] hover:text-text"
              aria-label="Clear search"
            >
              <X className="size-3" />
            </button>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto pb-2">
        {displayTree.length === 0 ? (
          <p className="px-3 py-4 text-center text-[12px] text-text-tertiary">
            No matches
          </p>
        ) : (
          displayTree.map((node) => (
            <TreeNode
              key={node.path}
              node={node}
              depth={0}
              selectedPath={selectedPath}
              onSelect={onSelect}
              expandedPaths={effectiveExpanded}
              onToggle={onToggle}
            />
          ))
        )}
      </div>
    </div>
  );
}
