import { useEffect, useState } from 'react';
import { useSearchParams, useNavigate } from 'react-router';
import {
  Lock,
  KeyRoundIcon,
  ArrowRight,
  UserX,
  Home,
  Loader2,
  LogIn,
} from 'lucide-react';
import { toast } from 'sonner';
import { useVerifyAppAccess } from '~/queries/apps';
import { Button } from '~/components/interface/button';
import { PasswordInput } from '~/components/interface/password-input';
import { Label } from '~/components/interface/label';
import { VStack } from '~/components/interface/stacks';
import { SlashaLogo } from '~/components/icons/slasha-logo';

type Props = {
  params: { slug: string };
};

export function meta() {
  return [{ title: 'Access Required · Slasha' }];
}

export default function AppAccessPage(props: Props) {
  const { slug } = props.params;
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const returnTo = searchParams.get('return_to') || '/';

  const verifyAccess = useVerifyAppAccess();
  const [password, setPassword] = useState('');
  const [forbiddenReason, setForbiddenReason] = useState<string | null>(null);
  const [unauthenticated, setUnauthenticated] = useState(false);
  const [checking, setChecking] = useState(true);

  const performVerify = async (pwd?: string) => {
    try {
      const result = await verifyAccess.mutateAsync({
        appSlug: slug,
        password: pwd,
      });

      const appOrigin = returnTo.startsWith('http')
        ? new URL(returnTo).origin
        : window.location.origin;

      const callbackUrl = `${appOrigin}/_slasha/app-access/callback?ticket=${encodeURIComponent(result.ticket)}&return_to=${encodeURIComponent(returnTo)}`;
      window.location.href = callbackUrl;
    } catch (e: any) {
      setChecking(false);
      if (e?.status === 401) {
        setUnauthenticated(true);
      } else if (e?.status === 403) {
        setForbiddenReason(
          e?.message || 'You are not a member of this application.'
        );
      } else if (pwd) {
        toast.error(
          e?.message || 'Incorrect password or unable to verify access.'
        );
      }
    }
  };

  useEffect(() => {
    performVerify();
  }, [slug]);

  const handleSubmit = (e: React.SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    setForbiddenReason(null);
    setUnauthenticated(false);
    performVerify(password);
  };

  return (
    <div className="relative flex min-h-dvh flex-col items-center justify-center overflow-hidden bg-bg px-4 py-12">
      <div
        className="pointer-events-none absolute left-1/2 top-1/2 size-[450px] -translate-x-1/2 -translate-y-1/2 rounded-full bg-gradient-to-tr from-white/[0.02] to-white/[0.06] blur-3xl"
        aria-hidden="true"
      />

      <div className="relative flex w-full max-w-[400px] flex-col items-center">
        <div className="mb-8 flex items-center justify-center">
          <SlashaLogo className="h-7 w-auto text-text" />
        </div>

        <div className="w-full rounded-2xl border border-border bg-surface/80 p-8 shadow-2xl backdrop-blur-xl">
          {checking ? (
            <div className="flex flex-col items-center py-6 text-center">
              <Loader2 className="mb-3 size-8 animate-spin text-text-secondary" />
              <p className="text-sm font-medium text-text">Verifying access…</p>
            </div>
          ) : unauthenticated ? (
            <div className="flex flex-col items-center text-center">
              <div className="mb-4 flex size-12 items-center justify-center rounded-2xl border border-amber-500/20 bg-amber-500/10 text-amber-400 shadow-inner">
                <Lock className="size-5" />
              </div>
              <h1 className="text-xl font-semibold tracking-tight text-text">
                Authentication Required
              </h1>
              <p className="mt-2 text-xs leading-relaxed text-text-tertiary">
                This application is private. You must log into Slasha to access
                it.
              </p>

              <VStack space={2} className="mt-6 w-full">
                <Button
                  onClick={() =>
                    navigate(
                      `/login?return_to=${encodeURIComponent(window.location.href)}`
                    )
                  }
                  label="Log In to Slasha"
                  icon={<LogIn className="size-4" />}
                  className="h-11 w-full justify-center bg-white text-bg font-medium hover:bg-white/90 focus:ring-0 focus:ring-offset-0"
                />
              </VStack>
            </div>
          ) : forbiddenReason ? (
            <div className="flex flex-col items-center text-center">
              <div className="mb-4 flex size-12 items-center justify-center rounded-2xl border border-red-500/20 bg-red-500/10 text-red-400 shadow-inner">
                <UserX className="size-5" />
              </div>
              <h1 className="text-xl font-semibold tracking-tight text-text">
                Access Denied
              </h1>
              <p className="mt-2 text-xs leading-relaxed text-text-tertiary">
                {forbiddenReason} Please ask the owner or administrator to add
                you to this application.
              </p>

              <VStack space={2} className="mt-6 w-full">
                <Button
                  onClick={() => navigate('/apps')}
                  label="Go to Dashboard"
                  icon={<Home className="size-4" />}
                  className="h-11 w-full justify-center bg-white text-bg font-medium hover:bg-white/90 focus:ring-0 focus:ring-offset-0"
                />
              </VStack>
            </div>
          ) : (
            <>
              <div className="mb-6 flex flex-col items-center text-center">
                <div className="mb-4 flex size-12 items-center justify-center rounded-2xl border border-border/80 bg-bg/60 text-text shadow-inner">
                  <Lock className="size-5 text-text-secondary" />
                </div>
                <h1 className="text-xl font-semibold tracking-tight text-text">
                  Access Required
                </h1>
                <p className="mt-1 text-xs leading-relaxed text-text-tertiary">
                  This application is password protected. Enter the password
                  below to continue.
                </p>
              </div>

              <form onSubmit={handleSubmit}>
                <VStack space={4}>
                  <VStack space={2}>
                    <Label
                      htmlFor="app-password"
                      className="text-[13px] font-medium text-text-secondary"
                    >
                      Password
                    </Label>
                    <div className="relative">
                      <KeyRoundIcon className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-tertiary" />
                      <PasswordInput
                        id="app-password"
                        placeholder="Enter password…"
                        value={password}
                        onChange={(e) => setPassword(e.target.value)}
                        className="h-11 border-border bg-bg/50 pl-9 text-text placeholder:text-text-tertiary transition-all focus-visible:border-text-secondary focus-visible:ring-0 text-[13px]"
                        autoFocus
                        required
                      />
                    </div>
                  </VStack>

                  <Button
                    type="submit"
                    label={verifyAccess.isPending ? 'Verifying…' : 'Continue'}
                    icon={<ArrowRight className="size-4" />}
                    isLoading={verifyAccess.isPending}
                    isDisabled={verifyAccess.isPending}
                    className="mt-1 h-11 w-full justify-center bg-white text-bg font-medium hover:bg-white/90 focus:ring-0 focus:ring-offset-0"
                  />
                </VStack>
              </form>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
