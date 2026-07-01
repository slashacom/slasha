import { useState } from 'react';
import {
  ExternalLink,
  GitBranch,
  Link as LinkIcon,
  Check,
  X,
} from 'lucide-react';
import type { App } from '~/models/app';
import type { GitAppConnection } from '~/queries/apps';
import {
  useGetRemoteBranchesQuery,
  useUpdateConnectionBranch,
} from '~/queries/connections';
import { BranchSelect } from './branch-select';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { toast } from 'sonner';

type Props = {
  app: App;
  connection?: GitAppConnection;
};

export function GitConnectionManager({ app, connection }: Props) {
  const [isEditingBranch, setIsEditingBranch] = useState(false);
  const [branchValue, setBranchValue] = useState(app.default_branch);

  const { data: remoteBranches, isFetching: branchesLoading } =
    useGetRemoteBranchesQuery(connection?.clone_url || '');

  const updateBranch = useUpdateConnectionBranch(app.slug);

  if (app.source !== 'git' || !connection) {
    return null;
  }

  const handleSaveBranch = async () => {
    if (!branchValue.trim()) return;
    try {
      await updateBranch.mutateAsync(branchValue.trim());
      setIsEditingBranch(false);
      toast.success('Successfully updated default branch');
    } catch (err: any) {
      toast.error(err.message || 'Failed to update branch');
    }
  };

  return (
    <div>
      <h3 className="text-[14px] font-semibold text-text">Git Repository</h3>
      <p className="mt-1 text-[13px] text-text-tertiary">
        Source repository mirrored by this application.
      </p>

      <div className="mt-6 border border-border bg-surface p-6">
        <div className="grid gap-5 sm:grid-cols-2">
          <div className="min-w-0">
            <div className="flex items-center gap-2 text-[12px] text-text-tertiary">
              <LinkIcon className="size-3.5" />
              Repository
            </div>
            <a
              href={connection.clone_url}
              target="_blank"
              rel="noreferrer"
              className="mt-1.5 flex min-w-0 items-center gap-1.5 text-[13px] font-medium text-text hover:underline"
            >
              <span className="truncate">{connection.clone_url}</span>
              <ExternalLink className="size-3.5 shrink-0 text-text-tertiary" />
            </a>
          </div>
          <div>
            <div className="flex items-center gap-2 text-[12px] text-text-tertiary">
              <GitBranch className="size-3.5" />
              Branch
            </div>
            {isEditingBranch ? (
              <div className="mt-1.5 flex items-center gap-2">
                <div className="relative flex-1 min-w-0">
                  {remoteBranches || branchesLoading ? (
                    <BranchSelect
                      branches={remoteBranches?.branches || []}
                      value={branchValue}
                      onChange={setBranchValue}
                      isLoading={branchesLoading}
                    />
                  ) : (
                    <Input
                      value={branchValue}
                      onChange={(e) => setBranchValue(e.target.value)}
                      className="h-8 text-[13px]"
                    />
                  )}
                </div>
                <Button
                  color="primary"
                  size="sm"
                  variant="ghost"
                  icon={<Check className="size-4" />}
                  onClick={handleSaveBranch}
                  isLoading={updateBranch.isPending}
                />
                <Button
                  color="neutral"
                  size="sm"
                  variant="ghost"
                  icon={<X className="size-4" />}
                  onClick={() => {
                    setIsEditingBranch(false);
                    setBranchValue(app.default_branch);
                  }}
                  isDisabled={updateBranch.isPending}
                />
              </div>
            ) : (
              <div className="mt-1.5 flex items-center gap-2">
                <p className="font-mono text-[13px] font-medium text-text">
                  {app.default_branch}
                </p>
                <button
                  type="button"
                  onClick={() => setIsEditingBranch(true)}
                  className="text-[12px] font-medium text-text-secondary hover:text-text hover:underline"
                >
                  Edit
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
