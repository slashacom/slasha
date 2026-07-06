import { MailIcon, KeyRoundIcon } from 'lucide-react';
import { redirect, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { VStack } from '~/components/interface/stacks';
import { SlashaLogo } from '~/components/icons/slasha-logo';
import { getAuthStatusOptions, useSignup } from '~/queries/auth';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  const status = await queryClient.ensureQueryData(getAuthStatusOptions());
  if (status.has_admin) {
    throw redirect('/login');
  }

  return null;
}

export function meta() {
  return [{ title: 'Set up Slasha' }];
}

export default function Signup() {
  const navigate = useNavigate();
  const signup = useSignup();

  const handleSubmit = (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const email = formData.get('email') as string;
    const password = formData.get('password') as string;
    const confirmPassword = formData.get('confirmPassword') as string;

    const promise = signup.mutateAsync({
      email,
      password,
      confirm_password: confirmPassword,
    });

    toast.promise(promise, {
      loading: 'Creating admin account...',
      success: () => {
        navigate('/apps');
        return `Welcome aboard, ${email}`;
      },
      error: (err) => err.message || 'Failed to set up.',
    });
  };

  return (
    <div className="flex w-full flex-col gap-8 py-10">
      <div className="flex flex-col items-start gap-6">
        <SlashaLogo className="h-9 w-auto text-text" />

        <div className="flex flex-col items-start gap-1.5">
          <p className="text-sm text-text-secondary">
            Create the first admin account. This account will own the instance
            and be able to invite others.
          </p>
        </div>
      </div>

      <form onSubmit={handleSubmit}>
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
                className="h-11 border-border bg-surface pl-9 text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
                placeholder="admin@slasha.app"
                autoComplete="email"
              />
            </div>
          </VStack>

          <VStack space={2}>
            <Label
              htmlFor="password"
              className="text-[13px] font-medium text-text-secondary"
            >
              Password
            </Label>
            <div className="relative">
              <KeyRoundIcon className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
              <Input
                id="password"
                name="password"
                type="password"
                required
                pattern=".{8,}"
                title="8 characters minimum"
                className="h-11 border-border bg-surface pl-9 text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
                placeholder="At least 8 characters"
                autoComplete="new-password"
              />
            </div>
            <p className="text-[12px] text-text-tertiary">
              Must be at least 8 characters.
            </p>
          </VStack>

          <VStack space={2}>
            <Label
              htmlFor="confirmPassword"
              className="text-[13px] font-medium text-text-secondary"
            >
              Confirm password
            </Label>
            <div className="relative">
              <KeyRoundIcon className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
              <Input
                id="confirmPassword"
                name="confirmPassword"
                type="password"
                required
                pattern=".{8,}"
                title="8 characters minimum"
                className="h-11 border-border bg-surface pl-9 text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
                placeholder="Confirm password"
                autoComplete="new-password"
              />
            </div>
          </VStack>

          <Button
            type="submit"
            isLoading={signup.isPending}
            isDisabled={signup.isPending}
            label={
              signup.isPending ? 'Creating account…' : 'Create admin account'
            }
            className="mt-2 h-11 w-full justify-center bg-white text-bg hover:bg-white/90 focus:ring-0 focus:ring-offset-0"
          />
        </VStack>
      </form>
    </div>
  );
}
