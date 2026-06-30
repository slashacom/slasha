import { useEffect, useRef, useState } from 'react';
import { ChevronDown, Globe, Lock, Search } from 'lucide-react';
import type { GithubRepository } from '~/queries/github';
import { cn } from '~/utils/classname';

interface RepositorySelectProps {
  repositories: GithubRepository[];
  value: string;
  onChange: (value: string) => void;
}

export function RepositorySelect({
  repositories,
  value,
  onChange,
}: RepositorySelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState('');
  const containerRef = useRef<HTMLDivElement>(null);

  // Close dropdown on click outside
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

  const selectedRepo = repositories.find((r) => r.id.toString() === value);

  const filteredRepos = repositories.filter((repo) =>
    repo.full_name.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div ref={containerRef} className="relative w-full">
      <button
        type="button"
        onClick={() => setIsOpen(!isOpen)}
        className={cn(
          'flex h-10 w-full items-center justify-between rounded-md border border-border bg-surface px-3 py-2 text-sm text-text outline-none transition-all hover:border-text-secondary focus:border-text-secondary',
          isOpen && 'border-text-secondary ring-1 ring-text-secondary/10'
        )}
      >
        {selectedRepo ? (
          <div className="flex items-center gap-2 truncate">
            {selectedRepo.private ? (
              <Lock className="size-3.5 text-text-secondary shrink-0" />
            ) : (
              <Globe className="size-3.5 text-text-secondary shrink-0" />
            )}
            <span className="font-medium truncate">
              {selectedRepo.full_name}
            </span>
          </div>
        ) : (
          <span className="text-text-tertiary">Select a repository...</span>
        )}
        <ChevronDown
          className={cn(
            'size-4 text-text-tertiary transition-transform duration-200',
            isOpen && 'rotate-180'
          )}
        />
      </button>

      {isOpen && (
        <div className="absolute z-50 mt-1.5 w-full rounded-md border border-border bg-surface shadow-lg animate-in fade-in-50 slide-in-from-top-1">
          <div className="relative border-b border-border p-2">
            <Search className="absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
            <input
              type="text"
              placeholder="Search repositories..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="h-9 w-full rounded-md bg-surface-secondary pl-9 pr-3 text-sm text-text placeholder:text-text-tertiary outline-none border-0 ring-0 focus:ring-1 focus:ring-text-secondary/20"
              autoFocus
            />
          </div>

          <ul className="max-h-60 overflow-y-auto p-1.5 space-y-0.5 scrollbar-thin">
            {filteredRepos.length === 0 ? (
              <li className="px-3 py-4 text-center text-xs text-text-tertiary">
                No repositories found
              </li>
            ) : (
              filteredRepos.map((repo) => {
                const isSelected = repo.id.toString() === value;
                return (
                  <li key={repo.id}>
                    <button
                      type="button"
                      onClick={() => {
                        onChange(repo.id.toString());
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
                        {repo.private ? (
                          <Lock className="size-3.5 text-text-tertiary shrink-0" />
                        ) : (
                          <Globe className="size-3.5 text-text-tertiary shrink-0" />
                        )}
                        <span className="truncate">{repo.full_name}</span>
                      </div>
                      <span className="text-[11px] text-text-tertiary font-mono bg-border/40 px-1.5 py-0.5 rounded">
                        {repo.default_branch}
                      </span>
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
