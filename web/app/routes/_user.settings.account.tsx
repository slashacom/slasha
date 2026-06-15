import { useQuery } from '@tanstack/react-query';
import { MailIcon, KeyRoundIcon } from 'lucide-react';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { VStack } from '~/components/interface/stacks';
import { getAuthMeOptions, useUpdateProfile } from '~/queries/auth';

export function meta() {
  return [{ title: 'Account Settings · slasha' }];
}

export default function AccountSettings() {
  const { data } = useQuery(getAuthMeOptions());
  const updateProfile = useUpdateProfile();

  const user = data?.user;

  const handleSubmit = (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const form = e.currentTarget;
    const formData = new FormData(form);
    const email = formData.get('email') as string;
    const currentPassword = formData.get('currentPassword') as string;
    const newPassword = formData.get('newPassword') as string;
    const confirmNewPassword = formData.get('confirmNewPassword') as string;

    const payload: Record<string, any> = {};

    if (email && email !== user?.email) {
      payload.email = email;
    }

    if (newPassword) {
      if (newPassword.length < 8) {
        toast.error('New password must be at least 8 characters');
        return;
      }
      if (newPassword !== confirmNewPassword) {
        toast.error('New passwords do not match');
        return;
      }
      payload.new_password = newPassword;
      payload.confirm_new_password = confirmNewPassword;
    }

    if (Object.keys(payload).length === 0) {
      toast.info('No changes to save');
      return;
    }

    if (!currentPassword) {
      toast.error('Current password is required to save changes');
      return;
    }

    payload.current_password = currentPassword;

    const promise = updateProfile.mutateAsync(payload);

    toast.promise(promise, {
      loading: 'Saving changes...',
      success: () => {
        form.reset();
        return 'Account settings updated successfully';
      },
      error: (err) => err.message || 'Failed to update settings.',
    });
  };

  return (
    <div className="space-y-6 max-w-xl">
      <div>
        <h3 className="font-semibold text-text">Account Settings</h3>
        <p className="mt-2 text-sm text-text-secondary">
          Manage your account profile and security settings.
        </p>
      </div>

      <form
        onSubmit={handleSubmit}
        className="border border-border bg-surface/20 rounded-lg p-6 space-y-6"
      >
        <VStack space={4}>
          <VStack space={2}>
            <Label
              htmlFor="email"
              className="text-[13px] font-medium text-text-secondary"
            >
              Email address
            </Label>
            <div className="relative">
              <MailIcon className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
              <Input
                id="email"
                name="email"
                type="email"
                required
                key={user?.email || ''}
                defaultValue={user?.email || ''}
                className="h-11 border-border bg-surface pl-9 text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
                placeholder="admin@slasha.app"
                autoComplete="email"
              />
            </div>
          </VStack>

          <VStack space={2}>
            <Label
              htmlFor="newPassword"
              className="text-[13px] font-medium text-text-secondary"
            >
              New Password
            </Label>
            <div className="relative">
              <KeyRoundIcon className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
              <Input
                id="newPassword"
                name="newPassword"
                type="password"
                pattern=".{8,}"
                title="8 characters minimum"
                className="h-11 border-border bg-surface pl-9 text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
                placeholder="New password"
                autoComplete="new-password"
              />
            </div>
            <p className="text-[12px] text-text-tertiary">
              Must be at least 8 characters.
            </p>
          </VStack>

          <VStack space={2}>
            <Label
              htmlFor="confirmNewPassword"
              className="text-[13px] font-medium text-text-secondary"
            >
              Confirm New Password
            </Label>
            <div className="relative">
              <KeyRoundIcon className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
              <Input
                id="confirmNewPassword"
                name="confirmNewPassword"
                type="password"
                className="h-11 border-border bg-surface pl-9 text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
                placeholder="Confirm new password"
                autoComplete="new-password"
              />
            </div>
          </VStack>

          <hr className="border-border my-2" />

          <VStack space={2}>
            <Label
              htmlFor="currentPassword"
              className="text-[13px] font-medium text-text-secondary"
            >
              Current Password
            </Label>
            <div className="relative">
              <KeyRoundIcon className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
              <Input
                id="currentPassword"
                name="currentPassword"
                type="password"
                required
                className="h-11 border-border bg-surface pl-9 text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
                placeholder="Required to save changes"
                autoComplete="current-password"
              />
            </div>
          </VStack>

          <Button
            type="submit"
            isLoading={updateProfile.isPending}
            isDisabled={updateProfile.isPending}
            label="Save changes"
            className="mt-2 h-11 w-full justify-center bg-white text-bg hover:bg-white/90 focus:ring-0 focus:ring-offset-0"
          />
        </VStack>
      </form>
    </div>
  );
}
