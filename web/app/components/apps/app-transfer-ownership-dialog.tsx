import { useEffect, useState } from 'react';
import { useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { AlertTriangle, Crown, LoaderCircle } from 'lucide-react';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '~/components/interface/dialog';
import { Select } from '~/components/interface/select';
import { Input } from '~/components/interface/input';
import { getAppMembersOptions, useTransferAppOwnership } from '~/queries/apps';
import { getUsersOptions } from '~/queries/users';
import { cn } from '~/utils/classname';

type AppTransferOwnershipDialogProps = {
  appSlug: string;
  appName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  preselectedUserId?: string;
};

export function AppTransferOwnershipDialog(
  props: AppTransferOwnershipDialogProps
) {
  const { appSlug, appName, open, onOpenChange, preselectedUserId } = props;
  const queryClient = useQueryClient();

  const { data: usersData } = useSuspenseQuery(getUsersOptions());
  const { data: membersData } = useSuspenseQuery(getAppMembersOptions(appSlug));

  const transferOwnership = useTransferAppOwnership();

  const members = membersData?.members ?? [];
  const currentOwner = members.find((m) => m.is_owner);
  const allUsers = usersData?.users ?? [];

  const eligibleUsers = allUsers.filter((u) => u.id !== currentOwner?.user_id);

  const [selectedUserId, setSelectedUserId] = useState('');
  const [confirmSlug, setConfirmSlug] = useState('');

  useEffect(() => {
    if (open) {
      if (
        preselectedUserId &&
        eligibleUsers.some((u) => u.id === preselectedUserId)
      ) {
        setSelectedUserId(preselectedUserId);
      } else {
        setSelectedUserId(eligibleUsers[0]?.id ?? '');
      }
      setConfirmSlug('');
    }
  }, [open, preselectedUserId, eligibleUsers.length]);

  const isConfirmed = confirmSlug.trim() === appSlug;

  const handleTransfer = async () => {
    if (!selectedUserId || !isConfirmed) return;

    try {
      await transferOwnership.mutateAsync({
        appSlug,
        user_id: selectedUserId,
      });
      toast.success('Application ownership transferred successfully');
      queryClient.invalidateQueries({ queryKey: ['apps'] });
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug] });
      queryClient.invalidateQueries({ queryKey: ['apps', appSlug, 'members'] });
      onOpenChange(false);
    } catch (err: any) {
      toast.error(err?.message || 'Failed to transfer application ownership');
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <div className="flex items-center gap-2 text-amber-400">
            <Crown className="size-4" />
            <DialogTitle>Transfer Ownership</DialogTitle>
          </div>
          <DialogDescription className="mt-1.5 text-[13px] leading-relaxed">
            Transfer ownership of{' '}
            <span className="font-medium text-text">{appName}</span> to another
            user.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-2">
          <div className="flex items-start gap-2.5 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-[12px] leading-relaxed text-amber-200/90">
            <AlertTriangle className="mt-0.5 size-4 shrink-0 text-amber-400" />
            <span>
              The new owner will have full administrative control over this
              application. If you are not a system administrator, you will
              become a team member with default access.
            </span>
          </div>

          <div>
            <label className="text-[12px] font-medium text-text-secondary">
              Select New Owner
            </label>
            <div className="mt-1.5">
              <Select
                value={selectedUserId}
                onChange={(e) => setSelectedUserId(e.target.value)}
              >
                {eligibleUsers.map((u) => (
                  <option key={u.id} value={u.id}>
                    {u.email}
                    {u.role === 'Admin' ? ' (Admin)' : ''}
                  </option>
                ))}
              </Select>
            </div>
          </div>

          <div>
            <label
              htmlFor="transfer-confirm-input"
              className="text-[12px] text-text-tertiary"
            >
              Type{' '}
              <span className="font-mono font-medium text-text">{appSlug}</span>{' '}
              to confirm
            </label>
            <Input
              id="transfer-confirm-input"
              value={confirmSlug}
              autoComplete="off"
              placeholder={appSlug}
              className="mt-1.5"
              onChange={(e) => setConfirmSlug(e.target.value)}
            />
          </div>
        </div>

        <DialogFooter className="gap-2 sm:gap-0">
          <button
            type="button"
            disabled={transferOwnership.isPending}
            onClick={() => onOpenChange(false)}
            className="cursor-pointer rounded-md px-3 py-1.5 text-[13px] text-text-secondary outline-none transition-colors hover:bg-white/5 hover:text-text disabled:cursor-not-allowed disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={
              !isConfirmed || !selectedUserId || transferOwnership.isPending
            }
            onClick={handleTransfer}
            className={cn(
              'inline-flex cursor-pointer items-center gap-1.5 rounded-md px-3 py-1.5 text-[13px] font-medium outline-none transition-colors disabled:cursor-not-allowed disabled:opacity-50',
              'bg-amber-600 text-white hover:bg-amber-500'
            )}
          >
            {transferOwnership.isPending ? (
              <LoaderCircle className="size-3.5 animate-spin" />
            ) : (
              <Crown className="size-3.5" />
            )}
            Transfer Ownership
          </button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
