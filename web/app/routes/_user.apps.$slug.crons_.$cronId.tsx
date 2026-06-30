import { useNavigate, useParams } from 'react-router';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { ArrowLeft, Clock, Pencil, Play } from 'lucide-react';
import { toast } from 'sonner';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import { Button } from '~/components/interface/button';
import { SectionHeader } from '~/components/interface/section-header';
import { CronRunHistory } from '~/components/apps/cron-run-history';
import { CronRunStatusBadge } from '~/components/apps/cron-run-status-badge';
import {
  getCronRunsOptions,
  getCronsOptions,
  useRunCron,
} from '~/queries/crons';
import { queryClient } from '~/utils/query-client';
import { formatDate } from '~/utils/format';

export async function clientLoader(args: {
  params: { slug: string; cronId: string };
}) {
  const { params } = args;
  await Promise.all([
    queryClient.ensureQueryData(getCronsOptions(params.slug)),
    queryClient.ensureQueryData(getCronRunsOptions(params.slug, params.cronId)),
  ]);
}

export default function CronDetailPage() {
  const { slug, cronId } = useParams();
  const navigate = useNavigate();
  const { data: cronsData } = useSuspenseQuery(getCronsOptions(slug!));
  const runCron = useRunCron(slug!);
  const { data: runsData } = useQuery({
    ...getCronRunsOptions(slug!, cronId!),
    refetchInterval: 5000,
  });
  const cron = cronsData.crons.find((item) => item.id === cronId);
  const latestRun = runsData?.runs?.[0];

  if (!cron) {
    return (
      <div className="p-8 text-sm text-text-secondary">Cron job not found.</div>
    );
  }

  const handleRun = () => {
    const promise = runCron.mutateAsync(cron.id);
    toast.promise(promise, {
      loading: 'Triggering run...',
      success: 'Run started.',
      error: (error) => error.message || 'Failed to start run.',
    });
  };

  const stats = [
    {
      label: 'Status',
      value: (
        <AlertStatusBadge state={cron.enabled ? 'ok' : 'muted'}>
          {cron.enabled ? 'Enabled' : 'Disabled'}
        </AlertStatusBadge>
      ),
    },
    {
      label: 'Next run',
      value: cron.enabled ? formatDate(cron.next_run_at) : '—',
    },
    {
      label: 'Last run',
      value: latestRun ? (
        <div className="flex items-center gap-2">
          <CronRunStatusBadge status={latestRun.status} />
          <span className="text-xs font-normal text-text-tertiary">
            {formatDate(latestRun.started_at ?? latestRun.created_at)}
          </span>
        </div>
      ) : (
        '—'
      ),
    },
    { label: 'Timeout', value: `${cron.timeout_secs}s` },
  ];

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-y-auto">
      <SectionHeader
        icon={Clock}
        title={cron.name}
        description={cron.schedule}
        actions={
          <>
            <Button
              to={`/apps/${slug}/crons`}
              label="Back"
              variant="ghost"
              icon={<ArrowLeft className="size-4" />}
            />
            <Button
              to={`/apps/${slug}/crons/${cron.id}/edit`}
              label="Edit"
              variant="ghost"
              icon={<Pencil className="size-4" />}
            />
            <Button
              label="Run now"
              icon={<Play className="size-4" />}
              onClick={handleRun}
              isLoading={runCron.isPending}
            />
          </>
        }
      />

      <div className="space-y-8 p-8">
        <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
          {stats.map((stat) => (
            <div
              key={stat.label}
              className="rounded-lg border border-border bg-surface p-5"
            >
              <p className="text-xs font-medium text-text-tertiary">
                {stat.label}
              </p>
              <div className="mt-2 text-sm font-semibold text-text">
                {stat.value}
              </div>
            </div>
          ))}
        </div>

        <div className="rounded-lg border border-border bg-surface p-6">
          <h3 className="text-xs font-medium text-text-tertiary">Command</h3>
          <pre className="mt-2 overflow-x-auto whitespace-pre-wrap break-words font-mono text-xs text-text-secondary">
            {cron.command}
          </pre>
          <p className="mt-3 text-[11px] text-text-tertiary">
            Runtime:{' '}
            {cron.runtime === 'utility' ? 'Utility (curl)' : 'App image'} ·
            Timezone: {cron.timezone}
          </p>
        </div>

        <div className="rounded-lg border border-border bg-surface p-6">
          <h3 className="mb-4 text-xs font-medium text-text-tertiary">
            Run history
          </h3>
          <CronRunHistory
            appSlug={slug!}
            cronId={cron.id}
            runs={runsData?.runs ?? []}
          />
        </div>
      </div>
    </div>
  );
}
