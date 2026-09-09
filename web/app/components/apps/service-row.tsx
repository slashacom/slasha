import { useNavigate } from 'react-router';
import type { Service } from '~/models/service';
import { HStack, VStack } from '~/components/interface/stacks';
import { StatusBadge } from '~/components/interface/status-badge';
import { CopyButton } from '~/components/interface/copy-button';
import { ServiceActionsMenu } from '~/components/apps/service-actions-menu';
import { ServiceKindBadge } from '~/components/apps/service-kind-badge';
import { describeResources } from '~/components/apps/service-resources';
import { formatRelativeTime } from '~/utils/date';
import { serviceEnvReference } from '~/utils/service-env';

type ServiceRowProps = {
  service: Service;
  appSlug: string;
};

function ServiceRowDetail(props: ServiceRowProps) {
  const { service } = props;

  if (service.status === 'Provisioning') {
    return (
      <span className="text-[11px] text-text-tertiary">
        Pulling the {service.kind} {service.version} image and starting the
        container.
      </span>
    );
  }

  if (service.status === 'Failed') {
    return (
      <span className="text-[11px] text-red-400/90">
        The container did not start. Open the service to read its logs, then
        redeploy.
      </span>
    );
  }

  if (service.status === 'Stopped') {
    return (
      <span className="text-[11px] text-text-tertiary">
        Stopped. Apps referencing its variables cannot connect until it is
        restarted.
      </span>
    );
  }

  const reference = serviceEnvReference(service.name, 'DATABASE_URL');

  return (
    <HStack space={1.5} className="min-w-0">
      <code className="truncate font-mono text-[11px] text-text-secondary">
        {reference}
      </code>
      <CopyButton
        value={reference}
        label={`Copy ${service.name} connection reference`}
        className="size-5"
      />
    </HStack>
  );
}

export function ServiceRow(props: ServiceRowProps) {
  const { service, appSlug } = props;
  const navigate = useNavigate();

  const limits = describeResources(service.resources).slice(0, 2).join(' · ');

  return (
    <div
      onClick={() => navigate(`/apps/${appSlug}/services/${service.id}`)}
      className="group grid cursor-pointer grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-4 px-8 py-4 transition-colors hover:bg-white/[0.02]"
    >
      <VStack space={1.5} className="min-w-0">
        <HStack space={3} className="min-w-0">
          <span className="truncate font-mono text-[13px] font-semibold text-text transition-colors group-hover:text-primary">
            {service.name}
          </span>
          <ServiceKindBadge service={service} />
          <StatusBadge status={service.status} />
        </HStack>
        <ServiceRowDetail service={service} appSlug={appSlug} />
      </VStack>

      <VStack space={1} className="hidden shrink-0 items-end lg:flex">
        <span className="whitespace-nowrap text-[11px] text-text-tertiary">
          Updated {formatRelativeTime(service.updated_at)}
        </span>
        {limits ? (
          <span className="whitespace-nowrap text-[11px] text-text-tertiary/70">
            {limits}
          </span>
        ) : null}
      </VStack>

      <ServiceActionsMenu appSlug={appSlug} service={service} variant="row" />
    </div>
  );
}
