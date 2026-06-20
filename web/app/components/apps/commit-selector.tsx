import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { CircleDashed, Search } from 'lucide-react';
import { getCommitsOptions } from '~/queries/deployments';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '~/components/interface/dialog';
import { Input } from '~/components/interface/input';
import { HStack, VStack } from '~/components/interface/stacks';

export function CommitSelector(props: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  appSlug: string;
  onSelect: (sha: string) => void;
  isDeploying: boolean;
}) {
  const { open, onOpenChange, appSlug, onSelect, isDeploying } = props;
  const { data, isLoading } = useQuery(getCommitsOptions(appSlug));
  const [search, setSearch] = useState('');
  const commits = data?.commits ?? [];

  const filteredCommits = commits.filter(
    (c) =>
      c.message.toLowerCase().includes(search.toLowerCase()) ||
      c.sha.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px] p-0 gap-0 overflow-hidden">
        <DialogHeader className="p-6 border-b border-border pb-4">
          <DialogTitle>Deploy Specific Commit</DialogTitle>
          <DialogDescription>
            Select a commit to trigger a new deployment.
          </DialogDescription>
        </DialogHeader>

        <div className="px-6 py-4 border-b border-border">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
            <Input
              placeholder="Search by message or SHA..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9 bg-surface"
              autoFocus
            />
          </div>
        </div>

        <div className="max-h-[400px] overflow-y-auto">
          {isLoading ? (
            <VStack className="p-8 items-center" space={4}>
              <CircleDashed className="size-6 animate-spin text-text-tertiary" />
              <p className="text-xs text-text-tertiary">Fetching commits...</p>
            </VStack>
          ) : filteredCommits.length === 0 ? (
            <VStack className="p-8 items-center" space={2}>
              <p className="text-sm text-text-secondary">No commits found</p>
              {search && (
                <p className="text-xs text-text-tertiary">
                  Try adjusting your search for "{search}"
                </p>
              )}
            </VStack>
          ) : (
            <div className="divide-y divide-border">
              {filteredCommits.map((commit) => (
                <button
                  key={commit.sha}
                  onClick={() => onSelect(commit.sha)}
                  disabled={isDeploying}
                  className="w-full text-left px-6 py-3 hover:bg-white/[0.02] transition-colors disabled:opacity-50 group"
                >
                  <VStack space={1}>
                    <HStack space={2} alignItems="center">
                      <span className="font-mono text-[12px] font-semibold text-text group-hover:text-primary transition-colors">
                        {commit.sha.slice(0, 7)}
                      </span>
                    </HStack>
                    <p className="text-[13px] text-text-secondary line-clamp-1">
                      {commit.message}
                    </p>
                  </VStack>
                </button>
              ))}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
