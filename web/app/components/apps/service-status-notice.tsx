import { AlertCircle, CircleDashed, Loader2, XCircle } from 'lucide-react';
import { toast } from 'sonner';
import type { Service } from '~/models/service';
import { useRedeployService, useRestartService } from '~/queries/services';
import { Button } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';

type ServiceStatusNoticeProps = {
  appSlug: string;
  service: Service;
};

export function ServiceStatusNotice(props: ServiceStatusNoticeProps) {
  const { appSlug, service } = props;
  const redeployService = useRedeployService();
  const restartService = useRestartService();

  const handleRedeploy = async () => {
    try {
      await redeployService.mutateAsync({ appSlug, serviceId: service.id });
      toast.success('Service redeploy started.');
    } catch (err) {
      toast.error('Failed to redeploy service: ' + err);
    }
  };

  const handleRestart = async () => {
    try {
      await restartService.mutateAsync({ appSlug, serviceId: service.id });
      toast.success('Service restart triggered.');
    } catch (err) {
      toast.error('Failed to restart service: ' + err);
    }
  };

  if (service.status === 'Provisioning') {
    return (
      <HStack
        space={3}
        alignItems="start"
        className="rounded-xl border border-sky-400/20 bg-sky-400/[0.06] px-4 py-3"
      >
        <CircleDashed className="mt-0.5 size-4 shrink-0 animate-spin text-sky-400" />
        <VStack space={1}>
          <span className="text-[13px] font-medium text-text">
            Provisioning
          </span>
          <span className="text-[12px] leading-5 text-text-tertiary">
            Pulling the {service.kind} {service.version} image and starting the
            container. This page refreshes on its own; the logs below stream the
            container&apos;s startup output.
          </span>
        </VStack>
      </HStack>
    );
  }

  if (service.status === 'Failed') {
    return (
      <HStack
        space={3}
        alignItems="start"
        className="rounded-xl border border-red-400/20 bg-red-400/[0.06] px-4 py-3"
      >
        <XCircle className="mt-0.5 size-4 shrink-0 text-red-400" />
        <VStack space={1} className="min-w-0 flex-1">
          <span className="text-[13px] font-medium text-text">
            Failed to start
          </span>
          <span className="text-[12px] leading-5 text-text-tertiary">
            The container exited before it became healthy. The logs below hold
            its last output — a bad variable, an unsupported version, or the
            node running out of memory are the usual causes. Fix the cause, then
            redeploy to recreate the container.
          </span>
        </VStack>
        <Button
          label="Redeploy"
          color="neutral"
          size="sm"
          icon={
            redeployService.isPending ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : undefined
          }
          onClick={handleRedeploy}
          isDisabled={redeployService.isPending}
        />
      </HStack>
    );
  }

  if (service.status === 'Stopped') {
    return (
      <HStack
        space={3}
        alignItems="start"
        className="rounded-xl border border-border bg-surface/50 px-4 py-3"
      >
        <AlertCircle className="mt-0.5 size-4 shrink-0 text-text-tertiary" />
        <VStack space={1} className="min-w-0 flex-1">
          <span className="text-[13px] font-medium text-text">Stopped</span>
          <span className="text-[12px] leading-5 text-text-tertiary">
            Its data volume is intact, but apps referencing its variables cannot
            connect until it is running again.
          </span>
        </VStack>
        <Button
          label="Restart"
          color="neutral"
          size="sm"
          icon={
            restartService.isPending ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : undefined
          }
          onClick={handleRestart}
          isDisabled={restartService.isPending}
        />
      </HStack>
    );
  }

  return null;
}
