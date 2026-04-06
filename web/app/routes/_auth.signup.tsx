import { redirect, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { VStack } from '~/components/interface/stacks';
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
  return [{ title: 'Sign up to Slasha' }];
}

export default function Signup() {
  const navigate = useNavigate();
  const signup = useSignup();

  const handleSubmit = (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const email = formData.get('email') as string;
    const password = formData.get('password') as string;

    const promise = signup.mutateAsync({ email, password });

    toast.promise(promise, {
      loading: 'Creating admin account...',
      success: () => {
        navigate('/');
        return `Successfully signed up as ${email}`;
      },
      error: (err) => err.message || 'Failed to sign up.',
    });
  };

  return (
    <div className="flex w-full flex-col gap-8 py-10">
      <VStack space={6}>
        <VStack space={1} alignItems="center">
          <h1 className="text-2xl font-bold tracking-tight text-neutral-900">
            Welcome to Slasha
          </h1>
          <p className="text-sm text-neutral-500">
            Create your admin account to get started
          </p>
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
                pattern=".{8,}"
                title="8 characters minimum"
                className="h-11 border-neutral-200 bg-neutral-50/30 transition-all focus-visible:border-black focus-visible:bg-white focus-visible:ring-0"
                placeholder="••••••••"
                autoComplete="new-password"
              />
            </VStack>

            <Button
              type="submit"
              isLoading={signup.isPending}
              isDisabled={signup.isPending}
              label={signup.isPending ? 'Creating Account...' : 'Continue'}
              className="mt-2 h-11 w-full justify-center"
            />
          </VStack>
        </form>
      </VStack>
    </div>
  );
}
