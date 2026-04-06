import { LockIcon } from 'lucide-react';
import { redirect, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { VStack, HStack } from '~/components/interface/stacks';
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
      <VStack space={6}>
        <VStack space={4} alignItems="center">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-neutral-50 border border-neutral-100">
            <LockIcon className="size-5 text-neutral-900" />
          </div>
          <VStack space={1} alignItems="center">
            <h1 className="text-2xl font-bold tracking-tight text-neutral-900">
              Welcome Back
            </h1>
            <p className="text-sm text-neutral-500">
              Sign in to your Slasha instance.
            </p>
          </VStack>
        </VStack>

        <form onSubmit={handleSubmit}>
          <VStack space={5}>
            <VStack space={2}>
              <Label
                htmlFor="email"
                className="text-sm font-medium text-neutral-700"
              >
                Email Address
              </Label>
              <Input
                id="email"
                name="email"
                type="email"
                required
                className="h-11 border-neutral-200 bg-neutral-50/30 transition-all focus-visible:border-black focus-visible:bg-white focus-visible:ring-0"
                placeholder="admin@slasha.app"
                autoComplete="email"
              />
            </VStack>

            <VStack space={2}>
              <Label
                htmlFor="password"
                className="text-sm font-medium text-neutral-700"
              >
                Password
              </Label>
              <Input
                id="password"
                name="password"
                type="password"
                required
                className="h-11 border-neutral-200 bg-neutral-50/30 transition-all focus-visible:border-black focus-visible:bg-white focus-visible:ring-0"
                placeholder="••••••••"
                autoComplete="current-password"
              />
            </VStack>

            <Button
              type="submit"
              isLoading={login.isPending}
              isDisabled={login.isPending}
              label={login.isPending ? 'Signing in...' : 'Sign In'}
              className="mt-2 h-11 w-full justify-center"
            />
          </VStack>
        </form>
      </VStack>
    </div>
  );
}
