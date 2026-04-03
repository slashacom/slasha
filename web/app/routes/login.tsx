import { Link, redirect, useNavigate } from 'react-router';
import { toast } from 'sonner';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { getAuthStatusOptions, useLogin } from '~/queries/auth';
import type { Route } from './+types/login';
import { queryClient } from '~/utils/query-client';
import { isLoggedIn } from '~/utils/jwt';

export async function clientLoader({ request }: Route.ClientLoaderArgs) {
  if (isLoggedIn()) {
    throw redirect('/');
  }

  const status = await queryClient.ensureQueryData(getAuthStatusOptions());
  if (!status.has_admin) {
    throw redirect('/signup');
  }

  return null;
}

export function meta({}: Route.MetaArgs) {
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
    <div className="flex min-h-screen items-center justify-center bg-[#fafafa] px-4 font-sans selection:bg-neutral-200">
      <div className="flex w-full max-w-[400px] flex-col gap-8 rounded-2xl border border-neutral-200/60 bg-white p-8 shadow-[0_8px_30px_rgb(0,0,0,0.04)] sm:p-10">
        <div className="flex flex-col items-center gap-3">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-neutral-100 shadow-inner">
            <svg
              className="size-6 text-black"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              viewBox="0 0 24 24"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z"
              />
            </svg>
          </div>
          <div className="text-center">
            <h1 className="text-2xl font-semibold tracking-tight text-neutral-900">
              Welcome Back
            </h1>
            <p className="mt-1.5 text-sm text-neutral-500">
              Sign in to your Slasha instance.
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
              type="email"
              required
              className="h-10 border-neutral-200 bg-neutral-50/50 transition-colors focus-visible:border-neutral-400 focus-visible:bg-white focus-visible:ring-0"
              placeholder="admin@slasha.app"
              autoComplete="email"
            />
          </div>

          <div className="flex flex-col gap-2">
            <div className="flex items-center justify-between">
              <Label htmlFor="password" className="text-neutral-700">
                Password
              </Label>
            </div>
            <Input
              id="password"
              name="password"
              type="password"
              required
              className="h-10 border-neutral-200 bg-neutral-50/50 transition-colors focus-visible:border-neutral-400 focus-visible:bg-white focus-visible:ring-0"
              placeholder="••••••••"
              autoComplete="current-password"
            />
          </div>

          <Button
            type="submit"
            isLoading={login.isPending}
            isDisabled={login.isPending}
            label={login.isPending ? 'Signing in...' : 'Sign In'}
            className="mt-2 w-full justify-center shadow-sm"
          />
        </form>
      </div>
    </div>
  );
}
