import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { ChevronUp, Pencil } from 'lucide-react';
import type { Service } from '~/models/service';
import { getServiceEnvVarsOptions } from '~/queries/services';
import { ServiceEnvEditor } from '~/components/apps/service-env-editor';
import { EnvVarChips } from '~/components/apps/env-var-chips';
import { HStack, VStack } from '~/components/interface/stacks';
import { primaryEnvKey, serviceEnvReference } from '~/utils/service-env';

type ServiceConfigCardProps = {
  appSlug: string;
  service: Service;
};

export function ServiceConfigCard(props: ServiceConfigCardProps) {
  const { appSlug, service } = props;
  const [open, setOpen] = useState(false);
  const { data: envData } = useQuery(
    getServiceEnvVarsOptions(appSlug, service.id)
  );

  if (open) {
    return (
      <div className="relative">
        <button
          type="button"
          onClick={() => setOpen(false)}
          aria-label="Collapse configuration"
          className="absolute right-5 top-5 z-10 flex size-7 cursor-pointer items-center justify-center rounded-md text-text-tertiary transition-colors hover:bg-white/5 hover:text-text"
        >
          <ChevronUp className="size-4" />
        </button>
        <ServiceEnvEditor
          appSlug={appSlug}
          serviceId={service.id}
          serviceName={service.name}
        />
      </div>
    );
  }

  const envKeys = Object.keys(envData?.env_vars ?? {}).sort();
  const reference = serviceEnvReference(service.name, primaryEnvKey(envKeys));

  return (
    <button
      type="button"
      onClick={() => setOpen(true)}
      className="group block w-full cursor-pointer rounded-xl border border-border bg-surface/50 p-4 text-left transition-colors hover:border-white/15 hover:bg-surface/70"
    >
      <HStack justifyContent="between" alignItems="start" space={4}>
        <VStack space={2} className="min-w-0">
          <span className="text-sm font-semibold text-text">Configuration</span>
          <EnvVarChips keys={envKeys} />
          <p className="text-[11px] leading-5 text-text-tertiary">
            Reference these from your app as{' '}
            <span className="font-mono text-text-secondary">{reference}</span>.
          </p>
        </VStack>
        <span className="flex size-7 shrink-0 items-center justify-center rounded-md text-text-tertiary transition-colors group-hover:bg-white/5 group-hover:text-text">
          <Pencil className="size-4" />
        </span>
      </HStack>
    </button>
  );
}
