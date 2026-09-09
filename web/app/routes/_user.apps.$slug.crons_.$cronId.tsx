import { useParams } from 'react-router';
import { useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { Pencil, Play } from 'lucide-react';
import { toast } from 'sonner';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import { Button } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';
import { Page } from '~/components/global/page';
import { PageHeader } from '~/components/interface/page-header';
import { TabActions } from '~/components/interface/tab-actions';
import { CronRunHistory } from '~/components/apps/cron-run-history';
import { CronRunStatusBadge } from '~/components/apps/cron-run-status-badge';
import {
  getCronRunsOptions,
  getCronsOptions,
  useRunCron,
} from '~/queries/crons';
import { queryClient } from '~/utils/query-client';
import { describeSchedule } from '~/utils/cron';
import {
  formatDateTime,
  formatRelativeTime,
  formatSeconds,
} from '~/utils/date';

export async function clientLoader(args: {
  params: { slug: string; cronId: string };
}) {
  const { params } = args;
  await Promise.all([
    queryClient.ensureQueryData(getCronsOptions(params.slug)),
    queryClient.ensureQueryData(getCronRunsOptions(params.slug, params.cronId)),
  ]);
}

type MetaItemProps = {
  label: string;
  children: React.ReactNode;
};

function MetaItem(props: MetaItemProps) {
  const { label, children } = props;

  return (
    <VStack space={1} className="min-w-0">
      <span className="text-[11px] font-medium uppercase tracking-wider text-text-tertiary">
        {label}
      </span>
      <div className="text-[13px] text-text">{children}</div>
    </VStack>
  );
}

export default function CronDetailPage() {
  const { slug, cronId } = useParams();
  const { data: cronsData } = useSuspenseQuery(getCronsOptions(slug!));
  const runCron = useRunCron(slug!);
  const { data: runsData } = useQuery({
    ...getCronRunsOptions(slug!, cronId!),
    refetchInterval: 5000,
  });
  const cron = cronsData.crons.find((item) => item.id === cronId);

  if (!cron) {
    return (
      <Page>
        <p className="text-sm text-text-secondary">Cron job not found.</p>
      </Page>
    );
  }

  const runs = runsData?.runs ?? [];
  const latestRun = runs[0];
  const description = describeSchedule(cron.schedule);

  const handleRun = () => {
    const promise = runCron.mutateAsync(cron.id);
    toast.promise(promise, {
      loading: 'Triggering run...',
      success: 'Run started.',
      error: (error) => error.message || 'Failed to start run.',
    });
  };

  return (
    <Page className="min-h-0 flex-1 overflow-y-auto">
      <TabActions>
        <Button
          to={`/apps/${slug}/crons/${cron.id}/edit`}
          label="Edit"
          variant="ghost"
          size="sm"
          icon={<Pencil className="size-3.5" />}
        />
        <Button
          label="Run now"
          size="sm"
          icon={<Play className="size-3.5" />}
          onClick={handleRun}
          isLoading={runCron.isPending}
        />
      </TabActions>

      <PageHeader
        title={cron.name}
        description={
          description ? `${description} · ${cron.timezone}` : cron.schedule
        }
      />

      <div className="mt-6 rounded-lg border border-border bg-surface">
        <HStack wrap alignItems="start" className="gap-x-12 gap-y-5 px-5 py-4">
          <MetaItem label="Status">
            <AlertStatusBadge state={cron.enabled ? 'ok' : 'muted'}>
              {cron.enabled ? 'Enabled' : 'Disabled'}
            </AlertStatusBadge>
          </MetaItem>
          <MetaItem label="Schedule">
            <span className="font-mono text-xs text-text-secondary">
              {cron.schedule}
            </span>
          </MetaItem>
          <MetaItem label="Next run">
            {cron.enabled ? formatDateTime(cron.next_run_at) : '—'}
          </MetaItem>
          <MetaItem label="Last run">
            {latestRun ? (
              <HStack space={2}>
                <CronRunStatusBadge status={latestRun.status} />
                <span className="text-text-tertiary">
                  {formatRelativeTime(
                    latestRun.started_at ?? latestRun.created_at
                  )}
                </span>
              </HStack>
            ) : (
              '—'
            )}
          </MetaItem>
          <MetaItem label="Timeout">
            {formatSeconds(cron.timeout_secs)}
          </MetaItem>
          <MetaItem label="Runtime">
            {cron.runtime === 'utility' ? 'Utility (curl)' : 'App image'}
          </MetaItem>
        </HStack>

        <div className="border-t border-border px-5 py-3">
          <MetaItem label="Command">
            <pre className="overflow-x-auto whitespace-pre-wrap break-words font-mono text-xs text-text-secondary">
              {cron.command}
            </pre>
          </MetaItem>
        </div>
      </div>

      <div className="mt-6 rounded-lg border border-border bg-surface p-6">
        <h3 className="mb-4 text-xs font-medium uppercase tracking-wider text-text-tertiary">
          Run history
        </h3>
        <CronRunHistory appSlug={slug!} cronId={cron.id} runs={runs} />
      </div>
    </Page>
  );
}
