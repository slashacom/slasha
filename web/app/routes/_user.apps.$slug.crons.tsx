import { useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Clock, Play, Plus } from 'lucide-react';
import { toast } from 'sonner';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { EmptyPage } from '~/components/global/empty-page';
import { SectionHeader } from '~/components/interface/section-header';
import { Table } from '~/components/interface/table';
import type { CronJob } from '~/models/cron';
import { getCronsOptions, useDeleteCron, useRunCron } from '~/queries/crons';
import { queryClient } from '~/utils/query-client';
import { formatDate } from '~/utils/format';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await queryClient.ensureQueryData(getCronsOptions(params.slug));
}

export default function AppCronsPage() {
  const { slug } = useParams();
  const navigate = useNavigate();
  const { data } = useSuspenseQuery(getCronsOptions(slug!));
  const deleteCron = useDeleteCron(slug!);
  const runCron = useRunCron(slug!);
  const [cronToDelete, setCronToDelete] = useState<CronJob | null>(null);

  const handleRun = (cron: CronJob) => {
    const promise = runCron.mutateAsync(cron.id);
    toast.promise(promise, {
      loading: `Triggering ${cron.name}...`,
      success: 'Run started.',
      error: (error) => error.message || 'Failed to start run.',
    });
  };

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-y-auto">
      <SectionHeader
        icon={Clock}
        title="Cron Jobs"
        actions={
          <Button
            to={`/apps/${slug}/crons/new`}
            label="New job"
            icon={<Plus className="size-4" />}
          />
        }
      />

      <div className="p-8">
        {data.crons.length === 0 ? (
          <EmptyPage
            icon={Clock}
            title="No cron jobs yet."
            subtitle="Schedule a command to run on a recurring basis against this app."
            actionLabel="Create job"
            actionIcon={<Plus className="size-4" />}
            onAction={() => navigate(`/apps/${slug}/crons/new`)}
            className="min-h-[320px]"
          />
        ) : (
          <div className="rounded-lg border border-border bg-surface p-6">
            <div className="overflow-x-auto">
              <Table
                columns={[
                  'Name',
                  'Schedule',
                  'Status',
                  'Next run',
                  { label: '', align: 'right' },
                ]}
              >
                {data.crons.map((cron) => (
                  <tr key={cron.id}>
                    <td className="py-3 pr-4">
                      <Link
                        to={`/apps/${slug}/crons/${cron.id}`}
                        className="font-medium text-text !no-underline hover:!underline"
                      >
                        {cron.name}
                      </Link>
                      <div className="mt-1 max-w-[280px] truncate font-mono text-xs text-text-tertiary">
                        {cron.command}
                      </div>
                    </td>
                    <td className="py-3 pr-4 font-mono text-text-secondary">
                      {cron.schedule}
                    </td>
                    <td className="py-3 pr-4">
                      <AlertStatusBadge state={cron.enabled ? 'ok' : 'muted'}>
                        {cron.enabled ? 'Enabled' : 'Disabled'}
                      </AlertStatusBadge>
                    </td>
                    <td className="py-3 pr-4 text-text-secondary">
                      {cron.enabled ? formatDate(cron.next_run_at) : '—'}
                    </td>
                    <td className="py-3 text-right">
                      <div className="flex items-center justify-end gap-3">
                        <button
                          type="button"
                          onClick={() => handleRun(cron)}
                          className="inline-flex items-center gap-1 text-xs text-text-secondary transition-colors hover:text-text"
                        >
                          <Play className="size-3" />
                          Run
                        </button>
                        <Link
                          to={`/apps/${slug}/crons/${cron.id}/edit`}
                          className="text-xs !text-text-secondary !no-underline hover:!text-text"
                        >
                          Edit
                        </Link>
                        <button
                          type="button"
                          onClick={() => setCronToDelete(cron)}
                          className="text-xs text-red-400 transition-colors hover:text-red-300"
                        >
                          Delete
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </Table>
            </div>
          </div>
        )}
      </div>

      <ConfirmationDialog
        open={cronToDelete !== null}
        onOpenChange={(open) => !open && setCronToDelete(null)}
        title="Delete cron job"
        description={
          cronToDelete
            ? `Delete ${cronToDelete.name}? This will stop all future runs.`
            : ''
        }
        confirmLabel="Delete"
        onConfirm={async () => {
          if (!cronToDelete) {
            return;
          }

          try {
            const promise = deleteCron.mutateAsync(cronToDelete.id);
            toast.promise(promise, {
              loading: 'Deleting job...',
              success: 'Job deleted.',
              error: (error) => error.message || 'Failed to delete job.',
            });
            await promise;
            setCronToDelete(null);
          } catch {
            return;
          }
        }}
      />
    </div>
  );
}
