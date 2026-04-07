import { MailIcon, KeyRoundIcon } from 'lucide-react';
import { redirect, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { VStack } from '~/components/interface/stacks';
import { getAuthStatusOptions, useLogin } from '~/queries/auth';
import { queryClient } from '~/utils/query-client';

export async function clientLoader() {
  const status = await queryClient.ensureQueryData(getAuthStatusOptions());
  if (!status.has_admin) {
    throw redirect('/signup');
  }

  return null;
}

export function meta() {
  return [{ title: 'Login to Slasha' }];
}

export default function Login() {
  const navigate = useNavigate();
  const login = useLogin();

  const handleSubmit = (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const email = formData.get('email') as string;
    const password = formData.get('password') as string;

    const promise = login.mutateAsync({ email, password });

    toast.promise(promise, {
      loading: 'Signing in...',
      success: () => {
        navigate('/');
        return `Welcome back, ${email}`;
      },
      error: (err) =>
        err.message || 'Failed to sign in. Please check your credentials.',
    });
  };

  return (
    <div className="flex w-full flex-col gap-8 py-10">
      <div className="flex flex-col items-start gap-1.5">
        <h1 className="text-2xl font-semibold tracking-tight text-text">
          Welcome back
        </h1>
        <p className="text-sm text-text-secondary">
          Sign in to your Slasha instance.
        </p>
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
                className="h-11 border-border bg-surface pl-9 text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0"
                placeholder="••••••••"
                autoComplete="current-password"
              />
            </div>
          </VStack>

          <Button
            type="submit"
            isLoading={login.isPending}
            isDisabled={login.isPending}
            label={login.isPending ? 'Signing in…' : 'Sign in'}
            className="mt-2 h-11 w-full justify-center bg-white text-bg hover:bg-white/90 focus:ring-0 focus:ring-offset-0"
          />
        </VStack>
      </form>
    </div>
  );
}
