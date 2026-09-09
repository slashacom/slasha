import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Database, Plus } from 'lucide-react';
import { getAppServicesOptions } from '~/queries/services';
import { Button } from '~/components/interface/button';
import { TabActions } from '~/components/interface/tab-actions';
import { EmptyPage } from '~/components/global/empty-page';
import { VStack } from '~/components/interface/stacks';
import { ServiceRow } from '~/components/apps/service-row';
import { ProvisionServiceModal } from '~/components/apps/provision-service-modal';

type ServicesViewProps = {
  appSlug: string;
};

export function ServicesView(props: ServicesViewProps) {
  const { appSlug } = props;
  const { data, isLoading } = useQuery({
    ...getAppServicesOptions(appSlug),
    refetchInterval: (query) => {
      const services = query.state.data?.services ?? [];
      const isAnyProvisioning = services.some(
        (s) => s.status === 'Provisioning'
      );
      return isAnyProvisioning ? 2000 : 5000;
    },
  });
  const [isProvisionModalOpen, setProvisionModalOpen] = useState(false);

  const services = data?.services ?? [];

  if (isLoading) {
    return (
      <VStack className="px-8 py-6" space={4}>
        <div className="h-4 w-32 animate-pulse rounded bg-white/[0.06]" />
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
      <TabActions>
        <Button
          label="Provision service"
          icon={<Plus className="size-3.5" />}
          size="sm"
          onClick={() => setProvisionModalOpen(true)}
        />
      </TabActions>

      {services.length === 0 ? (
        <EmptyPage
          className="mx-8 my-6 flex-1"
          icon={Database}
          title="No services running."
          subtitle="Provision a database, cache, or queue and attach it to this app with its credentials wired in."
          actionLabel="Provision service"
          actionIcon={<Plus className="size-3.5" />}
          onAction={() => setProvisionModalOpen(true)}
        />
      ) : (
        <div className="flex-1 overflow-auto">
          <div className="divide-y divide-border">
            {services.map((service) => (
              <ServiceRow
                key={service.id}
                service={service}
                appSlug={appSlug}
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
    </div>
  );
}
