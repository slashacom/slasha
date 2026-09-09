import { useState } from 'react';
import { useParams } from 'react-router';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Clock, Pencil, Play, Plus, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { AlertStatusBadge } from '~/components/alerts/alert-status-badge';
import { CronRunStatusBadge } from '~/components/apps/cron-run-status-badge';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { EmptyPage } from '~/components/global/empty-page';
import { TabActions } from '~/components/interface/tab-actions';
import { Table, TableRow } from '~/components/interface/table';
import { TableRowActions } from '~/components/interface/table-row-actions';
import type { CronJob } from '~/models/cron';
import { getCronsOptions, useDeleteCron, useRunCron } from '~/queries/crons';
import { queryClient } from '~/utils/query-client';
import { describeSchedule } from '~/utils/cron';
import { formatDateTime, formatRelativeTime } from '~/utils/date';

export async function clientLoader(args: { params: { slug: string } }) {
  const { params } = args;
  await queryClient.ensureQueryData(getCronsOptions(params.slug));
}

export default function AppCronsPage() {
  const { slug } = useParams();
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
      <TabActions>
        <Button
          to={`/apps/${slug}/crons/new`}
          label="New job"
          size="sm"
          icon={<Plus className="size-3.5" />}
        />
      </TabActions>

      <div className="px-8 py-6">
        {data.crons.length === 0 ? (
          <EmptyPage
            icon={Clock}
            size="lg"
            title="No scheduled jobs yet."
            subtitle="Cron jobs run a command against this app on a recurring schedule, like backups, digests and cleanups."
            actionLabel="Create job"
            actionIcon={<Plus className="size-3.5" />}
            actionTo={`/apps/${slug}/crons/new`}
          />
        ) : (
          <div className="-mx-8 overflow-x-auto">
            <Table
              columns={[
                'Name',
                'Schedule',
                'Status',
                'Last run',
                'Next run',
                { label: '', align: 'right' },
              ]}
            >
              {data.crons.map((cron) => (
                <TableRow key={cron.id} to={`/apps/${slug}/crons/${cron.id}`}>
                  <td className="py-3 pr-4">
                    <div className="font-medium text-text">{cron.name}</div>
                    <div className="mt-1 max-w-[280px] truncate font-mono text-xs text-text-tertiary">
                      {cron.command}
                    </div>
                  </td>
                  <td className="py-3 pr-4">
                    <div className="text-text-secondary">
                      {describeSchedule(cron.schedule) ?? cron.schedule}
                    </div>
                    <div className="mt-1 font-mono text-xs text-text-tertiary">
                      {cron.schedule}
                    </div>
                  </td>
                  <td className="py-3 pr-4">
                    <AlertStatusBadge state={cron.enabled ? 'ok' : 'muted'}>
                      {cron.enabled ? 'Enabled' : 'Disabled'}
                    </AlertStatusBadge>
                  </td>
                  <td className="py-3 pr-4">
                    {cron.last_run ? (
                      <>
                        <CronRunStatusBadge status={cron.last_run.status} />
                        <div className="mt-1 text-xs text-text-tertiary">
                          {formatRelativeTime(
                            cron.last_run.started_at ?? cron.last_run.created_at
                          )}
                        </div>
                      </>
                    ) : (
                      <span className="text-text-tertiary">—</span>
                    )}
                  </td>
                  <td className="py-3 pr-4 text-text-secondary">
                    {cron.enabled ? formatDateTime(cron.next_run_at) : '—'}
                  </td>
                  <td className="py-3 text-right">
                    <TableRowActions
                      actions={[
                        {
                          label: 'Run now',
                          icon: Play,
                          onClick: () => handleRun(cron),
                        },
                        {
                          label: 'Edit job',
                          icon: Pencil,
                          to: `/apps/${slug}/crons/${cron.id}/edit`,
                        },
                        {
                          label: 'Delete job',
                          icon: Trash2,
                          isDestructive: true,
                          onClick: () => setCronToDelete(cron),
                        },
                      ]}
                    />
                  </td>
                </TableRow>
              ))}
            </Table>
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
