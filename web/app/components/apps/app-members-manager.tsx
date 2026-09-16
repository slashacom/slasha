import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Crown, Loader2, Pencil, Plus, Trash2, Users } from 'lucide-react';
import { toast } from 'sonner';
import type { AppMemberPermissions, AppMemberWithUser } from '~/models/app';
import {
  getAppMembersOptions,
  useAddAppMember,
  useRemoveAppMember,
  useUpdateAppMember,
} from '~/queries/apps';
import { getAuthMeOptions } from '~/queries/auth';
import { getUsersOptions } from '~/queries/users';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '~/components/interface/dialog';
import { HStack } from '~/components/interface/stacks';
import { Select } from '~/components/interface/select';
import { SettingsCard } from '~/components/interface/settings-card';
import { Switch } from '~/components/interface/switch';
import { TableRowActions } from '~/components/interface/table-row-actions';
import { formatDate } from '~/utils/date';

type AppMembersManagerProps = {
  appSlug: string;
};

const DEFAULT_PERMISSIONS: AppMemberPermissions = {
  can_pull: true,
  can_push: true,
  can_deploy: false,
  can_manage_services: false,
  can_manage_settings: false,
  can_manage_members: false,
};

const ALL_PERMISSIONS: AppMemberPermissions = {
  can_pull: true,
  can_push: true,
  can_deploy: true,
  can_manage_services: true,
  can_manage_settings: true,
  can_manage_members: true,
};

const NO_PERMISSIONS: AppMemberPermissions = {
  can_pull: false,
  can_push: false,
  can_deploy: false,
  can_manage_services: false,
  can_manage_settings: false,
  can_manage_members: false,
};

const PERMISSION_CONFIG: {
  key: keyof AppMemberPermissions;
  label: string;
  description: string;
}[] = [
  {
    key: 'can_pull',
    label: 'Pull Repository',
    description: 'Read and clone code via Git HTTP/SSH and browse files.',
  },
  {
    key: 'can_push',
    label: 'Push Repository',
    description: 'Push commits to the repository via Git HTTP or SSH.',
  },
  {
    key: 'can_deploy',
    label: 'Deploy Application',
    description:
      'Trigger builds, deploy commits, roll back, and cancel builds.',
  },
  {
    key: 'can_manage_services',
    label: 'Manage Services',
    description:
      'Provision, configure, start, stop, and restart databases and services.',
  },
  {
    key: 'can_manage_settings',
    label: 'App Settings',
    description:
      'Manage environment variables, custom domains, crons, and backups.',
  },
  {
    key: 'can_manage_members',
    label: 'Manage Members',
    description: 'Add, modify, and remove team members and their permissions.',
  },
];

export function AppMembersManager(props: AppMembersManagerProps) {
  const { appSlug } = props;
  const queryClient = useQueryClient();

  const { data: membersData, isLoading: membersLoading } = useQuery(
    getAppMembersOptions(appSlug)
  );
  const { data: usersData } = useQuery(getUsersOptions());
  const { data: me } = useQuery(getAuthMeOptions());

  const addMember = useAddAppMember();
  const updateMember = useUpdateAppMember();
  const removeMember = useRemoveAppMember();

  const [addModalOpen, setAddModalOpen] = useState(false);
  const [selectedUserId, setSelectedUserId] = useState('');
  const [newPermissions, setNewPermissions] =
    useState<AppMemberPermissions>(DEFAULT_PERMISSIONS);

  const [editingMember, setEditingMember] = useState<AppMemberWithUser | null>(
    null
  );
  const [editPermissions, setEditPermissions] =
    useState<AppMemberPermissions>(DEFAULT_PERMISSIONS);

  const [removingMember, setRemovingMember] =
    useState<AppMemberWithUser | null>(null);

  const members = membersData?.members ?? [];
  const allUsers = usersData?.users ?? [];

  const existingMemberUserIds = new Set(members.map((m) => m.user_id));
  const availableUsers = allUsers.filter(
    (u) => !existingMemberUserIds.has(u.id)
  );

  const handleOpenAdd = () => {
    setSelectedUserId(availableUsers[0]?.id ?? '');
    setNewPermissions(DEFAULT_PERMISSIONS);
    setAddModalOpen(true);
  };

  const handleAddMember = async () => {
    if (!selectedUserId) {
      toast.error('Please select a user to add');
      return;
    }

    try {
      await addMember.mutateAsync({
        appSlug,
        user_id: selectedUserId,
        permissions: newPermissions,
      });
      toast.success('Team member added successfully');
      setAddModalOpen(false);
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'members'],
      });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to add team member');
    }
  };

  const handleOpenEdit = (member: AppMemberWithUser) => {
    setEditingMember(member);
    setEditPermissions({
      can_pull: member.can_pull,
      can_push: member.can_push,
      can_deploy: member.can_deploy,
      can_manage_services: member.can_manage_services,
      can_manage_settings: member.can_manage_settings,
      can_manage_members: member.can_manage_members,
    });
  };

  const handleUpdateMember = async () => {
    if (!editingMember) return;

    try {
      await updateMember.mutateAsync({
        appSlug,
        user_id: editingMember.user_id,
        permissions: editPermissions,
      });
      toast.success('Member permissions updated successfully');
      setEditingMember(null);
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'members'],
      });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to update member permissions');
    }
  };

  const handleConfirmRemove = async () => {
    if (!removingMember) return;

    try {
      await removeMember.mutateAsync({
        appSlug,
        user_id: removingMember.user_id,
      });
      toast.success('Team member removed successfully');
      setRemovingMember(null);
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'members'],
      });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to remove team member');
    }
  };

  const renderPermissionBadges = (member: AppMemberWithUser) => {
    if (member.is_owner) {
      return (
        <span className="inline-flex items-center gap-1.5 rounded border border-amber-500/25 bg-amber-500/10 px-2 py-0.5 text-xs font-medium text-amber-300">
          <Crown className="size-3 text-amber-400" />
          Owner
        </span>
      );
    }

    const allSelected =
      member.can_pull &&
      member.can_push &&
      member.can_deploy &&
      member.can_manage_services &&
      member.can_manage_settings &&
      member.can_manage_members;

    if (allSelected) {
      return (
        <span className="inline-flex items-center rounded border border-border bg-surface px-1.5 py-0.5 text-[11px] font-medium text-text-secondary">
          Full Access
        </span>
      );
    }

    const activePermissions = [
      { key: member.can_pull, label: 'Pull' },
      { key: member.can_push, label: 'Push' },
      { key: member.can_deploy, label: 'Deploy' },
      { key: member.can_manage_services, label: 'Services' },
      { key: member.can_manage_settings, label: 'Settings' },
      { key: member.can_manage_members, label: 'Members' },
    ].filter((p) => p.key);

    if (activePermissions.length === 0) {
      return <span className="text-xs text-text-tertiary">No access</span>;
    }

    return (
      <div className="flex flex-wrap gap-1.5">
        {activePermissions.map((p) => (
          <span
            key={p.label}
            className="inline-flex items-center rounded border border-border bg-surface px-1.5 py-0.5 text-[11px] font-medium text-text-secondary"
          >
            {p.label}
          </span>
        ))}
      </div>
    );
  };

  const body = (
    <div>
      {membersLoading ? (
        <div className="flex h-32 items-center justify-center text-text-tertiary">
          <Loader2 className="size-5 animate-spin" />
          <span className="ml-2">Loading team members...</span>
        </div>
      ) : members.length === 0 ? (
        <div className="p-8 text-center text-text-tertiary">
          <Users className="mx-auto mb-2 size-8 opacity-40" />
          <p className="text-sm">No team members added yet.</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-border bg-surface/30">
                <th className="py-3 pr-4 pl-6 text-xs font-medium uppercase tracking-wider text-text-tertiary">
                  Member
                </th>
                <th className="py-3 pr-4 text-xs font-medium uppercase tracking-wider text-text-tertiary">
                  Permissions
                </th>
                <th className="py-3 pr-4 text-xs font-medium uppercase tracking-wider text-text-tertiary">
                  Added
                </th>
                <th className="py-3 pr-6 text-right text-xs font-medium uppercase tracking-wider text-text-tertiary">
                  <span className="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {members.map((member) => (
                <tr
                  key={member.user_id}
                  className="group transition-colors hover:bg-white/[0.02]"
                >
                  <td className="py-3.5 pr-4 pl-6 align-middle">
                    <HStack space={2} alignItems="center">
                      <span className="font-medium text-text">
                        {member.email}
                      </span>
                      {member.user_id === me?.user?.id ? (
                        <span className="rounded border border-border bg-surface px-1.5 py-0.5 text-[11px] font-medium text-text-tertiary">
                          You
                        </span>
                      ) : null}
                    </HStack>
                  </td>
                  <td className="py-3.5 pr-4 align-middle">
                    {renderPermissionBadges(member)}
                  </td>
                  <td className="py-3.5 pr-4 align-middle text-xs whitespace-nowrap text-text-tertiary">
                    {formatDate(member.added_at)}
                  </td>
                  <td className="py-3.5 pr-6 align-middle text-right">
                    {member.is_owner ? null : (
                      <TableRowActions
                        actions={[
                          {
                            label: 'Edit permissions',
                            icon: Pencil,
                            onClick: () => handleOpenEdit(member),
                          },
                          {
                            label: 'Remove member',
                            icon: Trash2,
                            isDestructive: true,
                            onClick: () => setRemovingMember(member),
                          },
                        ]}
                      />
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );

  return (
    <>
      <SettingsCard
        icon={Users}
        title="Team Members"
        description="Manage team members and grant granular access permissions for repository operations, deployments, and app settings."
        actions={
          <Button
            color="neutral"
            size="sm"
            icon={<Plus className="size-3.5" />}
            label="Add Member"
            onClick={handleOpenAdd}
          />
        }
        body={body}
      />

      {/* Add Member Dialog */}
      <Dialog open={addModalOpen} onOpenChange={setAddModalOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Add Team Member</DialogTitle>
            <DialogDescription>
              Grant a user access to this application with custom permissions.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-2">
            <div className="space-y-1.5">
              <label
                htmlFor="member-user-select"
                className="text-[13px] font-medium text-text-secondary"
              >
                Select User
              </label>
              {availableUsers.length === 0 ? (
                <div className="rounded-md border border-border bg-surface/50 p-3 text-sm text-text-tertiary">
                  All registered users are already members of this application.
                </div>
              ) : (
                <Select
                  id="member-user-select"
                  value={selectedUserId}
                  onChange={(e) => setSelectedUserId(e.target.value)}
                >
                  {availableUsers.map((user) => (
                    <option key={user.id} value={user.id}>
                      {user.email} ({user.role})
                    </option>
                  ))}
                </Select>
              )}
            </div>

            <div className="space-y-3 pt-2">
              <div className="flex items-center justify-between">
                <span className="text-[13px] font-medium text-text-secondary">
                  Permissions
                </span>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => setNewPermissions(ALL_PERMISSIONS)}
                    className="cursor-pointer text-xs text-text-secondary transition-colors hover:text-text"
                  >
                    Grant all
                  </button>
                  <span className="text-xs text-text-tertiary">•</span>
                  <button
                    type="button"
                    onClick={() => setNewPermissions(NO_PERMISSIONS)}
                    className="cursor-pointer text-xs text-text-secondary transition-colors hover:text-text"
                  >
                    Clear all
                  </button>
                </div>
              </div>

              <div className="divide-y divide-border rounded-lg border border-border bg-surface/50">
                {PERMISSION_CONFIG.map((perm) => (
                  <div
                    key={perm.key}
                    className="flex items-start justify-between gap-4 p-3"
                  >
                    <div className="min-w-0 pr-2">
                      <p className="text-sm font-medium text-text">
                        {perm.label}
                      </p>
                      <p className="mt-0.5 text-xs text-text-tertiary">
                        {perm.description}
                      </p>
                    </div>
                    <Switch
                      checked={newPermissions[perm.key]}
                      onCheckedChange={(checked) =>
                        setNewPermissions((prev) => ({
                          ...prev,
                          [perm.key]: checked,
                        }))
                      }
                    />
                  </div>
                ))}
              </div>
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="ghost"
              label="Cancel"
              onClick={() => setAddModalOpen(false)}
            />
            <Button
              label="Add Member"
              isLoading={addMember.isPending}
              disabled={!selectedUserId || availableUsers.length === 0}
              onClick={handleAddMember}
            />
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Edit Member Dialog */}
      <Dialog
        open={Boolean(editingMember)}
        onOpenChange={(open) => !open && setEditingMember(null)}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Edit Permissions</DialogTitle>
            <DialogDescription>
              Configure access permissions for{' '}
              <span className="font-medium text-text">
                {editingMember?.email}
              </span>
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-2">
            <div className="flex items-center justify-between">
              <span className="text-[13px] font-medium text-text-secondary">
                Permissions
              </span>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setEditPermissions(ALL_PERMISSIONS)}
                  className="cursor-pointer text-xs text-text-secondary transition-colors hover:text-text"
                >
                  Grant all
                </button>
                <span className="text-xs text-text-tertiary">•</span>
                <button
                  type="button"
                  onClick={() => setEditPermissions(NO_PERMISSIONS)}
                  className="cursor-pointer text-xs text-text-secondary transition-colors hover:text-text"
                >
                  Clear all
                </button>
              </div>
            </div>

            <div className="divide-y divide-border rounded-lg border border-border bg-surface/50">
              {PERMISSION_CONFIG.map((perm) => (
                <div
                  key={perm.key}
                  className="flex items-start justify-between gap-4 p-3"
                >
                  <div className="min-w-0 pr-2">
                    <p className="text-sm font-medium text-text">
                      {perm.label}
                    </p>
                    <p className="mt-0.5 text-xs text-text-tertiary">
                      {perm.description}
                    </p>
                  </div>
                  <Switch
                    checked={editPermissions[perm.key]}
                    onCheckedChange={(checked) =>
                      setEditPermissions((prev) => ({
                        ...prev,
                        [perm.key]: checked,
                      }))
                    }
                  />
                </div>
              ))}
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="ghost"
              label="Cancel"
              onClick={() => setEditingMember(null)}
            />
            <Button
              label="Save Changes"
              isLoading={updateMember.isPending}
              onClick={handleUpdateMember}
            />
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Remove Member Confirmation */}
      <ConfirmationDialog
        open={Boolean(removingMember)}
        onOpenChange={(open) => !open && setRemovingMember(null)}
        title="Remove Team Member"
        description={`Are you sure you want to remove ${removingMember?.email} from this application? They will immediately lose access to repository code, deployments, and app settings.`}
        confirmLabel="Remove"
        isDestructive={true}
        isPending={removeMember.isPending}
        onConfirm={handleConfirmRemove}
      />
    </>
  );
}
