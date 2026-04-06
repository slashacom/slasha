import { Link, redirect, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { getAuthStatusOptions, useSignup } from '~/queries/auth';
import type { Route } from './+types/signup';
import { queryClient } from '~/utils/query-client';
import { isLoggedIn } from '~/utils/jwt';

export async function clientLoader({ request }: Route.ClientLoaderArgs) {
  if (isLoggedIn()) {
    throw redirect('/');
  }

  const status = await queryClient.ensureQueryData(getAuthStatusOptions());
  if (status.has_admin) {
    throw redirect('/login');
  }

  return null;
}

export function meta({}: Route.MetaArgs) {
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
    <div className="flex min-h-screen items-center justify-center bg-[#fafafa] px-4 font-sans selection:bg-neutral-200">
      <div className="flex w-full max-w-[400px] flex-col gap-8 rounded-2xl border border-neutral-200/60 bg-white p-8 shadow-[0_8px_30px_rgb(0,0,0,0.04)] sm:p-10">
        <div className="flex flex-col items-center gap-3">
          <div className="text-center">
            <h1 className="text-2xl font-semibold tracking-tight text-neutral-900">
              Welcome to Slasha
            </h1>
            <p className="mt-1.5 text-sm text-neutral-500">
              Create your admin account to get started
            </p>
          </div>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-6">
          <div className="flex flex-col gap-2">
            <Label htmlFor="email" className="text-neutral-700">
              Email
            </Label>
            <Input
              id="email"
              name="email"
              type="username"
              required
              className="h-10 border-neutral-200 bg-neutral-50/50 transition-colors focus-visible:border-neutral-400 focus-visible:bg-white focus-visible:ring-0"
              placeholder="admin@slasha.app"
              autoComplete="email"
            />
          </div>

          <div className="flex flex-col gap-2">
            <Label htmlFor="password" className="text-neutral-700">
              Password
            </Label>
            <Input
              id="password"
              name="password"
              type="password"
              required
              pattern=".{8,}"
              title="8 characters minimum"
              className="h-10 border-neutral-200 bg-neutral-50/50 transition-colors focus-visible:border-neutral-400 focus-visible:bg-white focus-visible:ring-0"
              placeholder="••••••••"
              autoComplete="new-password"
            />
          </div>

          <Button
            type="submit"
            isLoading={signup.isPending}
            isDisabled={signup.isPending}
            label={signup.isPending ? 'Creating Account...' : 'Continue'}
            className="mt-2 w-full justify-center shadow-sm"
          />
        </form>
      </div>
    </div>
  );
}
