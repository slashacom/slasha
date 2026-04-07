import { Link } from 'react-router';
import {
  ArrowRightIcon,
  GitBranchIcon,
  BoxIcon,
  ZapIcon,
  ShieldCheckIcon,
  ActivityIcon,
  CpuIcon,
  GaugeIcon,
  UnlockIcon,
  LayersIcon,
  ServerIcon,
  TerminalIcon,
  WorkflowIcon,
} from 'lucide-react';
import type { SVGProps } from 'react';
import { SlashaLogo } from '~/components/icons/slasha-logo';
import type { Route } from './+types/_index';

function GithubIcon(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 -0.5 25 25"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path d="m12.301 0h.093c2.242 0 4.34.613 6.137 1.68l-.055-.031c1.871 1.094 3.386 2.609 4.449 4.422l.031.058c1.04 1.769 1.654 3.896 1.654 6.166 0 5.406-3.483 10-8.327 11.658l-.087.026c-.063.02-.135.031-.209.031-.162 0-.312-.054-.433-.144l.002.001c-.128-.115-.208-.281-.208-.466 0-.005 0-.01 0-.014v.001q0-.048.008-1.226t.008-2.154c.007-.075.011-.161.011-.249 0-.792-.323-1.508-.844-2.025.618-.061 1.176-.163 1.718-.305l-.076.017c.573-.16 1.073-.373 1.537-.642l-.031.017c.508-.28.938-.636 1.292-1.058l.006-.007c.372-.476.663-1.036.84-1.645l.009-.035c.209-.683.329-1.468.329-2.281 0-.045 0-.091-.001-.136v.007c0-.022.001-.047.001-.072 0-1.248-.482-2.383-1.269-3.23l.003.003c.168-.44.265-.948.265-1.479 0-.649-.145-1.263-.404-1.814l.011.026c-.115-.022-.246-.035-.381-.035-.334 0-.649.078-.929.216l.012-.005c-.568.21-1.054.448-1.512.726l.038-.022-.609.384c-.922-.264-1.981-.416-3.075-.416s-2.153.152-3.157.436l.081-.02q-.256-.176-.681-.433c-.373-.214-.814-.421-1.272-.595l-.066-.022c-.293-.154-.64-.244-1.009-.244-.124 0-.246.01-.364.03l.013-.002c-.248.524-.393 1.139-.393 1.788 0 .531.097 1.04.275 1.509l-.01-.029c-.785.844-1.266 1.979-1.266 3.227 0 .025 0 .051.001.076v-.004c-.001.039-.001.084-.001.13 0 .809.12 1.591.344 2.327l-.015-.057c.189.643.476 1.202.85 1.693l-.009-.013c.354.435.782.793 1.267 1.062l.022.011c.432.252.933.465 1.46.614l.046.011c.466.125 1.024.227 1.595.284l.046.004c-.431.428-.718 1-.784 1.638l-.001.012c-.207.101-.448.183-.699.236l-.021.004c-.256.051-.549.08-.85.08-.022 0-.044 0-.066 0h.003c-.394-.008-.756-.136-1.055-.348l.006.004c-.371-.259-.671-.595-.881-.986l-.007-.015c-.198-.336-.459-.614-.768-.827l-.009-.006c-.225-.169-.49-.301-.776-.38l-.016-.004-.32-.048c-.023-.002-.05-.003-.077-.003-.14 0-.273.028-.394.077l.007-.003q-.128.072-.08.184c.039.086.087.16.145.225l-.001-.001c.061.072.13.135.205.19l.003.002.112.08c.283.148.516.354.693.603l.004.006c.191.237.359.505.494.792l.01.024.16.368c.135.402.38.738.7.981l.005.004c.3.234.662.402 1.057.478l.016.002c.33.064.714.104 1.106.112h.007c.045.002.097.002.15.002.261 0 .517-.021.767-.062l-.027.004.368-.064q0 .609.008 1.418t.008.873v.014c0 .185-.08.351-.208.466h-.001c-.119.089-.268.143-.431.143-.075 0-.147-.011-.214-.032l.005.001c-4.929-1.689-8.409-6.283-8.409-11.69 0-2.268.612-4.393 1.681-6.219l-.032.058c1.094-1.871 2.609-3.386 4.422-4.449l.058-.031c1.739-1.034 3.835-1.645 6.073-1.645h.098-.005zm-7.64 17.666q.048-.112-.112-.192-.16-.048-.208.032-.048.112.112.192.144.096.208-.032zm.497.545q.112-.08-.032-.256-.16-.144-.256-.048-.112.08.032.256.159.157.256.047zm.48.72q.144-.112 0-.304-.128-.208-.272-.096-.144.08 0 .288t.272.112zm.672.673q.128-.128-.064-.304-.192-.192-.32-.048-.144.128.064.304.192.192.32.044zm.913.4q.048-.176-.208-.256-.24-.064-.304.112t.208.24q.24.097.304-.096zm1.009.08q0-.208-.272-.176-.256 0-.256.176 0 .208.272.176.256.001.256-.175zm.929-.16q-.032-.176-.288-.144-.256.048-.224.24t.288.128.225-.224z" />
    </svg>
  );
}

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
        className="pointer-events-none absolute inset-x-0 top-0 -z-10 h-[900px] [background-image:radial-gradient(hsl(0_0%_100%/4%)_1px,transparent_1px)] [background-size:22px_22px] [mask-image:radial-gradient(ellipse_at_top,black_20%,transparent_70%)]"
      />

      <Header />
      <Hero />
      <Features />
      <HowItWorks />
      <FinalCta />
      <Footer />
    </div>
  );
}

function Header() {
  return (
    <header className="sticky top-0 z-20 border-b border-border/60 bg-bg/80 backdrop-blur">
      <div className="mx-auto flex w-full max-w-6xl items-center justify-between px-6 py-4">
        <Link to="/" className="!no-underline">
          <SlashaLogo className="h-8 w-auto text-text" />
        </Link>
        <nav className="flex items-center gap-2">
          <a
            href="https://github.com"
            target="_blank"
            rel="noreferrer"
            className="hidden h-9 items-center gap-2 rounded-md border border-border bg-surface px-3 font-mono text-[12px] !text-text-secondary !no-underline transition-colors hover:!text-text sm:inline-flex"
          >
            <GithubIcon className="size-3.5" />
            GitHub
          </a>
          <Link
            to="/login"
            className="inline-flex h-9 items-center gap-2 rounded-md bg-white px-4 text-[12.5px] font-medium !text-bg !no-underline transition-colors hover:bg-white/90"
          >
            Get started
            <ArrowRightIcon className="size-3.5" />
          </Link>
        </nav>
      </div>
    </header>
  );
}

function Hero() {
  return (
    <section className="relative px-6 pt-20 pb-24 sm:pt-28">
      <div className="mx-auto grid w-full max-w-6xl gap-14 lg:grid-cols-[1.1fr_1fr] lg:items-center lg:gap-12">
        <div className="text-left">
          <span className="mb-6 inline-flex items-center gap-2 rounded-full border border-border bg-surface px-3 py-1 font-mono text-[10.5px] uppercase tracking-[0.16em] text-text-secondary">
            <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
            v0.1 · open source
          </span>

          <h1 className="text-balance text-[40px] font-semibold leading-[1.05] tracking-tight text-text sm:text-[52px]">
            Just{' '}
            <span className="rounded-lg bg-surface px-2.5 py-0.5 font-mono text-[34px] text-text sm:text-[44px]">
              git push
            </span>
            .<br />
            slasha does the rest.
          </h1>

          <p className="mt-6 max-w-[520px] text-[15.5px] leading-relaxed text-text-secondary">
            A self-hostable, open-source PaaS built on Docker. Point a remote
            at your own server and ship — no dashboards, no YAML, no lock-in.
            Heroku-grade DX on infrastructure you control.
          </p>

          <div className="mt-8 flex flex-col items-start gap-3 sm:flex-row sm:items-center">
            <Link
              to="/login"
              className="group inline-flex h-11 items-center gap-2 rounded-md bg-white px-5 text-[13px] font-medium !text-bg !no-underline transition-colors hover:bg-white/90"
            >
              Get started
              <ArrowRightIcon className="size-3.5 transition-transform group-hover:translate-x-0.5" />
            </Link>
            <a
              href="https://github.com"
              target="_blank"
              rel="noreferrer"
              className="inline-flex h-11 items-center gap-2 rounded-md border border-border bg-surface px-5 text-[13px] font-medium !text-text-secondary !no-underline transition-colors hover:bg-white/5 hover:!text-text"
            >
              <GithubIcon className="size-4" />
              View on GitHub
            </a>
          </div>

          <div className="mt-8 flex flex-wrap items-center gap-x-5 gap-y-2 font-mono text-[11px] uppercase tracking-[0.12em] text-text-tertiary">
            <span className="inline-flex items-center gap-1.5">
              <span className="h-1 w-1 rounded-full bg-text-tertiary" />
              MIT licensed
            </span>
            <span className="inline-flex items-center gap-1.5">
              <span className="h-1 w-1 rounded-full bg-text-tertiary" />
              Single binary
            </span>
            <span className="inline-flex items-center gap-1.5">
              <span className="h-1 w-1 rounded-full bg-text-tertiary" />
              No vendor lock-in
            </span>
          </div>
        </div>

        <div className="relative">
          <div
            aria-hidden
            className="absolute -inset-6 -z-10 rounded-2xl bg-gradient-to-br from-white/[0.04] to-transparent blur-2xl"
          />
          <TerminalCard />
        </div>
      </div>
    </section>
  );
}

function TerminalCard() {
  return (
    <div className="overflow-hidden rounded-xl border border-border bg-code-bg text-left shadow-2xl shadow-black/40">
      <div className="flex items-center gap-1.5 border-b border-border px-3.5 py-2.5">
        <span className="h-2.5 w-2.5 rounded-full bg-white/10" />
        <span className="h-2.5 w-2.5 rounded-full bg-white/10" />
        <span className="h-2.5 w-2.5 rounded-full bg-white/10" />
        <span className="ml-2 font-mono text-[11px] text-text-tertiary">
          ~/my-app
        </span>
      </div>
      <pre className="overflow-x-auto px-4 py-4 font-mono text-[12.5px] leading-[1.75] text-code-text">
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
        <span className="text-text-tertiary">→ detected Dockerfile</span>
        {'\n'}
        <span className="text-text-tertiary">→ building image…</span>
        {'\n'}
        <span className="text-text-tertiary">→ provisioning micro-vm…</span>
        {'\n'}
        <span className="text-text-tertiary">→ routing traffic…</span>
        {'\n'}
        <span className="text-emerald-400">
          ✓ live at my-app.my-server.com
        </span>
      </pre>
    </div>
  );
}

function SectionEyebrow(props: { children: React.ReactNode }) {
  return (
    <span className="inline-flex items-center gap-2 font-mono text-[11px] uppercase tracking-[0.18em] text-text-tertiary">
      <span className="h-px w-6 bg-border" />
      {props.children}
    </span>
  );
}

function Features() {
  return (
    <section className="border-t border-border/60 px-6 py-24">
      <div className="mx-auto w-full max-w-6xl">
        <div className="mb-14 max-w-2xl">
          <SectionEyebrow>Features</SectionEyebrow>
          <h2 className="mt-4 text-[32px] font-medium leading-[1.15] tracking-tight text-text sm:text-[40px]">
            A whole platform,
            <br />
            <span className="text-text-tertiary">in a single binary.</span>
          </h2>
          <p className="mt-4 text-[15px] leading-relaxed text-text-secondary">
            slasha bundles the things you'd cobble together yourself —
            builds, isolation, scaling, monitoring — into a single binary
            that runs on any Linux box.
          </p>
        </div>

        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 lg:grid-rows-[auto_auto_auto_auto_auto]">
          <BigCard />

          <FeatureCard
            icon={<BoxIcon className="size-4" />}
            title="Docker native"
            description="Bring your own Dockerfile. If it builds locally, it ships."
          />
          <FeatureCard
            icon={<GitBranchIcon className="size-4" />}
            title="Git push deploys"
            description="No CLI, no dashboard. Push to a remote and you're live."
          />

          <FeatureCard
            icon={<GaugeIcon className="size-4" />}
            title="Auto-scaling"
            description="Scale containers up and down based on real traffic."
          />
          <FeatureCard
            icon={<UnlockIcon className="size-4" />}
            title="No vendor lock-in"
            description="Plain Docker images. Move them anywhere, anytime."
          />
          <FeatureCard
            icon={<ShieldCheckIcon className="size-4" />}
            title="Secure by default"
            description="Automatic TLS, isolated networks, signed builds."
          />

          <WideCard
            icon={<CpuIcon className="size-4" />}
            title="Micro-VM isolation"
            description="Every app runs in its own lightweight VM. Strong tenant isolation without container-escape risk — boots in milliseconds."
            visual={<MicroVmVisual />}
          />
          <FeatureCard
            icon={<ActivityIcon className="size-4" />}
            title="Monitoring built-in"
            description="Logs, metrics, and traces wired up out of the box."
          />

          <FeatureCard
            icon={<ZapIcon className="size-4" />}
            title="Highly optimized"
            description="Fast cold starts and minimal overhead per app."
          />
          <FeatureCard
            icon={<ServerIcon className="size-4" />}
            title="Long-running services"
            description="Workers, cron, queues, and websockets — first class."
          />
          <FeatureCard
            icon={<LayersIcon className="size-4" />}
            title="Full stack"
            description="Web, API, workers, databases, and storage in one place."
          />
        </div>
      </div>
    </section>
  );
}

function BigCard() {
  return (
    <div className="relative col-span-1 row-span-2 flex flex-col justify-between overflow-hidden rounded-xl border border-border bg-surface p-6 sm:col-span-2 lg:col-span-2 lg:row-span-2">
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 [background-image:radial-gradient(hsl(0_0%_100%/5%)_1px,transparent_1px)] [background-size:18px_18px] [mask-image:radial-gradient(ellipse_at_top_right,black_20%,transparent_70%)]"
      />

      <div className="relative">
        <div className="inline-flex items-center gap-2 rounded-full border border-border bg-bg/60 px-2.5 py-1 font-mono text-[10px] uppercase tracking-[0.14em] text-text-secondary">
          <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />
          Open source
        </div>
        <h3 className="mt-5 text-[26px] font-medium leading-[1.15] tracking-tight text-text">
          Free, MIT-licensed,
          <br />
          and yours forever.
        </h3>
        <p className="mt-3 max-w-md text-[14px] leading-relaxed text-text-secondary">
          Self-host on any Linux machine — a $5 VPS, a homelab, or a fleet of
          bare-metal boxes. No seats, no metering, no surprise bills. Read the
          code, fork it, run it.
        </p>
      </div>

      <div className="relative mt-8 flex items-end justify-between gap-4">
        <a
          href="https://github.com"
          target="_blank"
          rel="noreferrer"
          className="inline-flex items-center gap-2 font-mono text-[12px] !text-text !no-underline"
        >
          <GithubIcon className="size-4" />
          github.com/slasha
          <ArrowRightIcon className="size-3.5" />
        </a>
        <div className="hidden sm:block">
          <CommitGraph />
        </div>
      </div>
    </div>
  );
}

function CommitGraph() {
  const heights = [18, 26, 14, 32, 22, 40, 28, 36, 24, 44, 30, 48];
  return (
    <div className="flex items-end gap-1">
      {heights.map((h, i) => (
        <span
          key={i}
          className="w-1.5 rounded-sm bg-white/10"
          style={{ height: `${h}px` }}
        />
      ))}
    </div>
  );
}

function FeatureCard(props: {
  icon: React.ReactNode;
  title: string;
  description: string;
}) {
  const { icon, title, description } = props;
  return (
    <div className="group relative flex flex-col overflow-hidden rounded-xl border border-border bg-surface p-5 transition-colors hover:border-white/15">
      <div className="flex size-8 items-center justify-center rounded-md border border-border bg-bg/60 text-text-secondary group-hover:text-text">
        {icon}
      </div>
      <h3 className="mt-4 text-[14.5px] font-medium text-text">{title}</h3>
      <p className="mt-1.5 text-[13px] leading-relaxed text-text-secondary">
        {description}
      </p>
    </div>
  );
}

function WideCard(props: {
  icon: React.ReactNode;
  title: string;
  description: string;
  visual: React.ReactNode;
}) {
  const { icon, title, description, visual } = props;
  return (
    <div className="relative col-span-1 flex flex-col justify-between overflow-hidden rounded-xl border border-border bg-surface p-5 sm:col-span-2 lg:col-span-2">
      <div>
        <div className="flex size-8 items-center justify-center rounded-md border border-border bg-bg/60 text-text-secondary">
          {icon}
        </div>
        <h3 className="mt-4 text-[14.5px] font-medium text-text">{title}</h3>
        <p className="mt-1.5 max-w-md text-[13px] leading-relaxed text-text-secondary">
          {description}
        </p>
      </div>
      <div className="mt-5">{visual}</div>
    </div>
  );
}

function MicroVmVisual() {
  return (
    <div className="grid grid-cols-4 gap-2">
      {Array.from({ length: 8 }).map((_, i) => (
        <div
          key={i}
          className="flex h-10 items-center justify-between rounded-md border border-border bg-bg/60 px-2 font-mono text-[9px] text-text-tertiary"
        >
          <span className="truncate">vm-{(i + 1).toString().padStart(2, '0')}</span>
          <span className="size-1.5 rounded-full bg-emerald-400/70" />
        </div>
      ))}
    </div>
  );
}

function HowItWorks() {
  const steps = [
    {
      n: '01',
      icon: <ServerIcon className="size-4" />,
      title: 'Install on your server',
      description:
        'One command on any Linux box. slasha runs as a single binary — no Kubernetes, no Helm, no node groups.',
      code: 'curl -fsSL slasha.dev/install | sh',
    },
    {
      n: '02',
      icon: <GitBranchIcon className="size-4" />,
      title: 'Add a git remote',
      description:
        'Point your repo at your slasha server. Works with any project that has a Dockerfile.',
      code: 'git remote add slasha git@srv:app.git',
    },
    {
      n: '03',
      icon: <WorkflowIcon className="size-4" />,
      title: 'Push to deploy',
      description:
        'slasha builds the image, provisions a micro-vm, wires up TLS, and routes traffic. Done.',
      code: 'git push slasha main',
    },
  ];

  return (
    <section className="border-t border-border/60 px-6 py-24">
      <div className="mx-auto w-full max-w-6xl">
        <div className="mb-14 max-w-2xl">
          <SectionEyebrow>How it works</SectionEyebrow>
          <h2 className="mt-4 text-[32px] font-medium leading-[1.15] tracking-tight text-text sm:text-[40px]">
            Three steps.
            <br />
            <span className="text-text-tertiary">No YAML in sight.</span>
          </h2>
        </div>

        <div className="grid gap-3 lg:grid-cols-3">
          {steps.map((step) => (
            <div
              key={step.n}
              className="relative flex flex-col overflow-hidden rounded-xl border border-border bg-surface p-6"
            >
              <div className="flex items-center justify-between">
                <span className="font-mono text-[11px] uppercase tracking-[0.18em] text-text-tertiary">
                  Step {step.n}
                </span>
                <div className="flex size-8 items-center justify-center rounded-md border border-border bg-bg/60 text-text-secondary">
                  {step.icon}
                </div>
              </div>
              <h3 className="mt-5 text-[18px] font-medium tracking-tight text-text">
                {step.title}
              </h3>
              <p className="mt-2 text-[13.5px] leading-relaxed text-text-secondary mb-5">
                {step.description}
              </p>
              <div className="mt-auto flex items-center gap-2 rounded-md border border-border bg-code-bg px-3 py-2.5 font-mono text-[11.5px] text-code-text">
                <TerminalIcon className="size-3.5 shrink-0 text-text-tertiary" />
                <span className="truncate">{step.code}</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function FinalCta() {
  return (
    <section className="border-t border-border/60 px-6 py-24">
      <div className="relative mx-auto w-full max-w-6xl overflow-hidden rounded-2xl border border-border bg-surface px-6 py-16 text-center sm:px-12">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 [background-image:radial-gradient(hsl(0_0%_100%/5%)_1px,transparent_1px)] [background-size:22px_22px] [mask-image:radial-gradient(ellipse_at_center,black_10%,transparent_65%)]"
        />
        <div className="relative">
          <SectionEyebrow>Ship faster</SectionEyebrow>
          <h2 className="mx-auto mt-4 max-w-2xl text-balance text-[34px] font-medium leading-[1.1] tracking-tight text-text sm:text-[44px]">
            Stop babysitting your infra.
            <br />
            <span className="text-text-tertiary">Start shipping.</span>
          </h2>
          <p className="mx-auto mt-5 max-w-xl text-[15px] leading-relaxed text-text-secondary">
            Self-host slasha in five minutes. Deploy your first app in
            another five.
          </p>
          <div className="mt-8 flex flex-col items-center justify-center gap-3 sm:flex-row">
            <Link
              to="/login"
              className="group inline-flex h-11 items-center gap-2 rounded-md bg-white px-5 text-[13px] font-medium !text-bg !no-underline transition-colors hover:bg-white/90"
            >
              Get started
              <ArrowRightIcon className="size-3.5 transition-transform group-hover:translate-x-0.5" />
            </Link>
            <a
              href="https://github.com"
              target="_blank"
              rel="noreferrer"
              className="inline-flex h-11 items-center gap-2 rounded-md border border-border bg-bg px-5 text-[13px] font-medium !text-text-secondary !no-underline transition-colors hover:bg-white/5 hover:!text-text"
            >
              <GithubIcon className="size-4" />
              Star on GitHub
            </a>
          </div>
        </div>
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="border-t border-border/60 px-6 py-10">
      <div className="mx-auto flex w-full max-w-6xl flex-col items-center justify-between gap-4 sm:flex-row">
        <SlashaLogo className="h-6 w-auto text-text-tertiary" />
        <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-text-tertiary">
          © {new Date().getFullYear()} slasha · built for self-hosters
        </p>
      </div>
    </footer>
  );
}
