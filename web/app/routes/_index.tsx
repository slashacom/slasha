import { Link } from 'react-router';
import { ArrowRightIcon } from 'lucide-react';
import { SlashaLogo } from '~/components/icons/slasha-logo';
import type { Route } from './+types/_index';

export function meta({}: Route.MetaArgs) {
  return [
    { title: 'slasha — self-hosted PaaS for developers' },
    {
      name: 'description',
      content:
        'A self-hostable, open-source PaaS. Just git push — slasha builds, ships, and scales your apps on your own infrastructure.',
    },
  ];
}

export default function Index() {
  return (
    <div className="relative flex min-h-screen flex-col bg-bg text-text">
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 -z-10 [background-image:radial-gradient(hsl(0_0%_100%/4%)_1px,transparent_1px)] [background-size:22px_22px] [mask-image:radial-gradient(ellipse_at_center,black_30%,transparent_75%)]"
      />

      <header className="flex items-center justify-center px-6 py-6">
        <SlashaLogo className="h-4 w-auto text-text" />
      </header>

      <main className="flex flex-1 items-center justify-center px-6 pb-20">
        <div className="flex w-full max-w-[600px] flex-col items-center text-center">
          <span className="mb-7 inline-flex items-center gap-2 rounded-full border border-border bg-surface px-3 py-1 text-[11px] font-medium uppercase tracking-[0.14em] text-text-secondary">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
            Open source · Self-hosted
          </span>

          <h1 className="text-balance text-[34px] font-medium leading-[1.1] tracking-tight text-text">
            Just{' '}
            <code className="rounded-md bg-surface px-2 py-0.5 font-mono text-[28px] text-text">
              git push
            </code>
            .<br />
            slasha does the rest.
          </h1>

          <p className="mt-5 max-w-[480px] text-balance text-[15px] leading-relaxed text-text-secondary">
            A self-hostable, open-source PaaS built on Docker. Point a remote
            at your own server and ship — no dashboards, no YAML, no lock-in.
          </p>

          <div className="mt-8 flex flex-col items-center gap-3 sm:flex-row">
            <Link
              to="/login"
              className="group inline-flex h-10 items-center gap-2 rounded-md bg-white px-5 text-[13px] font-medium !text-bg !no-underline transition-colors hover:bg-white/90"
            >
              Get started
              <ArrowRightIcon className="size-3.5 transition-transform group-hover:translate-x-0.5" />
            </Link>
            <a
              href="https://github.com"
              target="_blank"
              rel="noreferrer"
              className="inline-flex h-10 items-center gap-2 rounded-md border border-border bg-surface px-5 text-[13px] font-medium !text-text-secondary !no-underline transition-colors hover:bg-white/5 hover:!text-text"
            >
              View on GitHub
            </a>
          </div>

          <div className="mt-10 w-full overflow-hidden rounded-lg border border-border bg-code-bg text-left">
            <div className="flex items-center gap-1.5 border-b border-border px-3 py-2">
              <span className="h-2.5 w-2.5 rounded-full bg-white/10" />
              <span className="h-2.5 w-2.5 rounded-full bg-white/10" />
              <span className="h-2.5 w-2.5 rounded-full bg-white/10" />
              <span className="ml-2 text-[11px] text-text-tertiary">
                ~/my-app
              </span>
            </div>
            <pre className="overflow-x-auto px-4 py-3.5 font-mono text-[12.5px] leading-[1.7] text-code-text">
              <span className="text-text-tertiary">
                # add your slasha server as a remote
              </span>
              {'\n'}
              <span className="text-text-tertiary">$ </span>
              <span className="text-text">
                git remote add slasha git@my-server.com:my-app.git
              </span>
              {'\n\n'}
              <span className="text-text-tertiary"># ship it</span>
              {'\n'}
              <span className="text-text-tertiary">$ </span>
              <span className="text-text">git push slasha main</span>
              {'\n'}
              <span className="text-text-tertiary">
                → detected Dockerfile
              </span>
              {'\n'}
              <span className="text-text-tertiary">
                → building &amp; deploying…
              </span>
              {'\n'}
              <span className="text-emerald-400">
                ✓ live at my-app.my-server.com
              </span>
            </pre>
          </div>
        </div>
      </main>

      <footer className="px-6 py-6 text-center text-[12px] text-text-tertiary">
        © {new Date().getFullYear()} slasha · built for developers who
        self-host
      </footer>
    </div>
  );
}
