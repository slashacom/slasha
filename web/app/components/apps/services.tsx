import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Database, Server, Plus } from 'lucide-react';
import { getAppServicesOptions } from '~/queries/services';
import { Button } from '~/components/interface/button';
import { SectionHeader } from '~/components/interface/section-header';
import { VStack } from '~/components/interface/stacks';
import { ServiceRow } from '~/components/apps/service-row';
import { ProvisionServiceModal } from '~/components/apps/provision-service-modal';
import { LogStreamDialog } from '~/components/apps/log-stream-dialog';

type ServicesViewProps = {
  appSlug: string;
};

export function ServicesView(props: ServicesViewProps) {
  const { appSlug } = props;
  const { data, isLoading } = useQuery(getAppServicesOptions(appSlug));
  const [isProvisionModalOpen, setProvisionModalOpen] = useState(false);
  const [activeLogsId, setActiveLogsId] = useState<{
    id: string;
    name: string;
  } | null>(null);

  const services = data?.services ?? [];

  if (isLoading) {
    return (
      <VStack className="p-8" space={4}>
        <div className="h-4 w-32 animate-pulse rounded bg-surface-hover" />
        <VStack space={2}>
          {[1, 2].map((i) => (
            <div
              key={i}
              className="h-16 w-full animate-pulse rounded border border-border bg-surface"
            />
          ))}
        </VStack>
      </VStack>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-1 flex-col overflow-hidden">
      <SectionHeader
        icon={Server}
        title="Service Infrastructure"
        actions={
          <Button
            label="Provision Service"
            icon={<Plus className="size-3.5" />}
            size="sm"
            onClick={() => setProvisionModalOpen(true)}
          />
        }
      />

      {services.length === 0 ? (
        <VStack className="flex-1 items-center justify-center" space={4}>
          <div className="rounded-full border border-border p-4">
            <Database className="size-8 text-text-tertiary" />
          </div>
          <VStack alignItems="center" space={1}>
            <p className="text-sm font-medium text-text">No services running</p>
            <p className="text-xs text-text-tertiary text-center max-w-[280px]">
              Provision databases and auxiliary services to attach them to your
              application.
            </p>
          </VStack>
          <Button
            label="Provision First Service"
            size="sm"
            onClick={() => setProvisionModalOpen(true)}
          />
        </VStack>
      ) : (
        <div className="flex-1 overflow-auto">
          <div className="divide-y divide-border">
            {services.map((service) => (
              <ServiceRow
                key={service.id}
                service={service}
                appSlug={appSlug}
                onShowLogs={() =>
                  setActiveLogsId({ id: service.id, name: service.name })
                }
              />
            ))}
          </div>
        </div>
      )}

      {isProvisionModalOpen && (
        <ProvisionServiceModal
          appSlug={appSlug}
          onClose={() => setProvisionModalOpen(false)}
        />
      )}
      {activeLogsId && (
        <LogStreamDialog
          url={`/api/apps/${appSlug}/services/${activeLogsId.id}/logs`}
          title={`Service Logs — ${activeLogsId.name}`}
          onClose={() => setActiveLogsId(null)}
        />
      )}
    </div>
  );
}
