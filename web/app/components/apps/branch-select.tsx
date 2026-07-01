import { useEffect, useRef, useState } from 'react';
import { ChevronDown, GitBranch, Search } from 'lucide-react';
import { cn } from '~/utils/classname';

interface BranchSelectProps {
  branches: string[];
  value: string;
  onChange: (value: string) => void;
  isLoading?: boolean;
}

export function BranchSelect({
  branches,
  value,
  onChange,
  isLoading,
}: BranchSelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState('');
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        containerRef.current &&
        !containerRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const filteredBranches = branches.filter((branch) =>
    branch.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div ref={containerRef} className="relative w-full">
      <button
        type="button"
        onClick={() => !isLoading && setIsOpen(!isOpen)}
        disabled={isLoading}
        className={cn(
          'flex h-10 w-full items-center justify-between rounded-md border border-border bg-surface px-3 py-2 text-sm text-text outline-none transition-all hover:border-text-secondary focus:border-text-secondary disabled:cursor-not-allowed disabled:opacity-60',
          isOpen && 'border-text-secondary ring-1 ring-text-secondary/10'
        )}
      >
        {isLoading ? (
          <span className="text-text-tertiary">Loading branches...</span>
        ) : value ? (
          <div className="flex items-center gap-2 truncate">
            <GitBranch className="size-3.5 text-text-secondary shrink-0" />
            <span className="font-mono text-[13px] truncate">{value}</span>
          </div>
        ) : (
          <span className="text-text-tertiary">Select a branch...</span>
        )}
        <ChevronDown
          className={cn(
            'size-4 text-text-tertiary transition-transform duration-200',
            isOpen && 'rotate-180'
          )}
        />
      </button>

      {isOpen && !isLoading && (
        <div className="absolute z-50 mt-1.5 w-full rounded-md border border-border bg-surface shadow-lg animate-in fade-in-50 slide-in-from-top-1">
          <div className="relative border-b border-border p-2">
            <Search className="absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
            <input
              type="text"
              placeholder="Search branches..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="h-9 w-full rounded-md bg-surface-secondary pl-9 pr-3 text-sm text-text placeholder:text-text-tertiary outline-none border-0 ring-0 focus:ring-1 focus:ring-text-secondary/20 font-mono"
              autoFocus
            />
          </div>

          <ul className="max-h-60 overflow-y-auto p-1.5 space-y-0.5 scrollbar-thin">
            {filteredBranches.length === 0 ? (
              <li className="px-3 py-4 text-center text-xs text-text-tertiary">
                No branches found
              </li>
            ) : (
              filteredBranches.map((branch) => {
                const isSelected = branch === value;
                return (
                  <li key={branch}>
                    <button
                      type="button"
                      onClick={() => {
                        onChange(branch);
                        setIsOpen(false);
                        setSearch('');
                      }}
                      className={cn(
                        'flex w-full items-center justify-between rounded px-3 py-2 text-left text-sm transition-colors hover:bg-surface-secondary',
                        isSelected &&
                          'bg-surface-secondary text-text font-medium'
                      )}
                    >
                      <div className="flex items-center gap-2 truncate">
                        <GitBranch className="size-3.5 text-text-tertiary shrink-0" />
                        <span className="font-mono text-[13px] truncate">
                          {branch}
                        </span>
                      </div>
                    </button>
                  </li>
                );
              })
            )}
          </ul>
        </div>
      )}
    </div>
  );
}
