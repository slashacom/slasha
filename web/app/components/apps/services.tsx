import { useState, useMemo, useEffect, useRef } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Database,
  Play,
  Square,
  Clock,
  AlertCircle,
  CheckCircle2,
  XCircle,
  CircleDashed,
  Terminal,
  Trash2,
  Server,
  Plus,
  Settings,
} from 'lucide-react';
import type { Service, ServiceStatus, ServiceKind } from '~/models/service';
import {
  getServiceKindsOptions,
  getAppServicesOptions,
  useProvisionService,
  useStopService,
  useDeleteService,
} from '~/queries/services';
import { Button } from '~/components/interface/button';
import { ConfirmationDialog } from '~/components/interface/confirmation-dialog';
import { HStack, VStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import { formatRelativeTime } from '~/utils/format';
import { getAuthToken } from '~/utils/jwt';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '~/components/interface/dialog';
import { TextInput } from '~/components/interface/text-input';
import { EnvEditor, ServiceEnvEditor } from '~/components/apps/env-editor';

export function ServicesView({ appSlug }: { appSlug: string }) {
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
      <HStack
        justifyContent="between"
        className="border-b border-border px-8 py-4"
      >
        <HStack space={2}>
          <Server className="size-4 text-text-tertiary" />
          <h2 className="text-sm font-semibold text-text">
            Service Infrastructure
          </h2>
        </HStack>
        <Button
          label="Provision Service"
          icon={<Plus className="size-3.5" />}
          size="sm"
          onClick={() => setProvisionModalOpen(true)}
        />
      </HStack>

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
        <ServiceLogModal
          serviceId={activeLogsId.id}
          serviceName={activeLogsId.name}
          appSlug={appSlug}
          onClose={() => setActiveLogsId(null)}
        />
      )}
    </div>
  );
}

function StatusBadge({ status }: { status: ServiceStatus }) {
  const configs: Record<
    ServiceStatus,
    { icon: any; color: string; bg: string }
  > = {
    Provisioning: {
      icon: CircleDashed,
      color: 'text-sky-400',
      bg: 'bg-sky-400/10',
    },
    Running: {
      icon: CheckCircle2,
      color: 'text-emerald-400',
      bg: 'bg-emerald-400/10',
    },
    Failed: { icon: XCircle, color: 'text-red-400', bg: 'bg-red-400/10' },
    Stopped: {
      icon: AlertCircle,
      color: 'text-text-tertiary',
      bg: 'bg-white/5',
    },
  };

  const { icon: Icon, color, bg } = configs[status];

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded px-2 py-0.5 text-[11px] font-medium',
        color,
        bg
      )}
    >
      <Icon
        className={cn('size-3', status === 'Provisioning' && 'animate-spin')}
      />
      {status}
    </span>
  );
}

function ServiceRow({
  service,
  appSlug,
  onShowLogs,
}: {
  service: Service;
  appSlug: string;
  onShowLogs: () => void;
}) {
  const queryClient = useQueryClient();
  const stopService = useStopService();
  const deleteService = useDeleteService();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [showConfig, setShowConfig] = useState(false);

  const handleStop = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await stopService.mutateAsync({
        appSlug,
        serviceId: service.id,
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services'],
      });
    } catch (err) {
      toast.error('Failed to stop service: ' + err);
    }
  };

  const handleDelete = async () => {
    try {
      await deleteService.mutateAsync({
        appSlug,
        serviceId: service.id,
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services'],
      });
      setShowDeleteConfirm(false);
    } catch (e) {
      toast.error('Failed to delete service: ' + e);
    }
  };

  return (
    <>
      <div className="group grid grid-cols-[1fr_auto] items-center gap-4 px-8 py-4 transition-colors hover:bg-white/[0.02]">
        <VStack space={1.5}>
          <HStack space={3}>
            <span className="font-mono text-[13px] font-semibold text-text">
              {service.name}
            </span>
            <span className="text-[11px] font-medium text-text-secondary bg-surface-hover px-1.5 py-0.5 rounded">
              {service.kind} {service.version}
            </span>
            <StatusBadge status={service.status} />
          </HStack>
          <HStack space={3}>
            <span className="text-[11px] text-text-tertiary">
              slasha-svc-{service.id.slice(0, 8)}
            </span>
            <span className="text-[11px] text-text-tertiary">
              Created {formatRelativeTime(service.created_at)}
            </span>
          </HStack>
        </VStack>

        <HStack space={2}>
          <Button
            label="Logs"
            icon={<Terminal className="size-3.5" />}
            variant="ghost"
            size="sm"
            color="neutral"
            onClick={onShowLogs}
          />
          {(service.status === 'Running' ||
            service.status === 'Provisioning') && (
            <Button
              label="Stop"
              icon={<Square className="size-3.5" />}
              variant="ghost"
              size="sm"
              color="error"
              onClick={handleStop}
              isLoading={stopService.isPending}
            />
          )}
          <Button
            label="Settings"
            icon={<Settings className="size-3.5" />}
            variant="ghost"
            size="sm"
            onClick={(e) => {
              e.stopPropagation();
              setShowConfig(true);
            }}
          />
          <Button
            label="Delete"
            icon={<Trash2 className="size-3.5" />}
            variant="ghost"
            size="sm"
            color="error"
            onClick={(e) => {
              e.stopPropagation();
              setShowDeleteConfirm(true);
            }}
            isLoading={deleteService.isPending}
          />
        </HStack>
      </div>

      <ConfirmationDialog
        open={showDeleteConfirm}
        onOpenChange={setShowDeleteConfirm}
        title="Delete Service"
        description={`Are you sure you want to delete ${service.name}? All underlying data will be permanently destroyed.`}
        confirmLabel="Delete Service"
        onConfirm={handleDelete}
      />

      {showConfig && (
        <ServiceConfigModal
          appSlug={appSlug}
          service={service}
          onClose={() => setShowConfig(false)}
        />
      )}
    </>
  );
}

function ProvisionServiceModal({
  appSlug,
  onClose,
}: {
  appSlug: string;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const { data } = useQuery(getServiceKindsOptions());
  const provisionService = useProvisionService();

  const kinds = data?.kinds ?? [];

  const [name, setName] = useState('');
  const [kindName, setKindName] = useState<ServiceKind | ''>('');
  const [version, setVersion] = useState<string>('');
  const [envVars, setEnvVars] = useState<Record<string, string>>({});

  const selectedKind = useMemo(() => {
    return kinds.find((k) => k.name === kindName);
  }, [kinds, kindName]);

  const handleKindChange = (newKind: ServiceKind) => {
    setKindName(newKind);
    const kindMeta = kinds.find((k) => k.name === newKind);
    if (kindMeta) {
      setVersion(kindMeta.supported_versions[0]);
      setEnvVars(kindMeta.default_env_vars);
    }
  };

  useEffect(() => {
    if (kinds.length > 0 && !kindName) {
      handleKindChange(kinds[0].name);
    }
  }, [kinds, kindName]);

  const handleProvision = async () => {
    if (!name.trim() || !kindName || !version) {
      toast.error('Please fill in all fields.');
      return;
    }
    try {
      await provisionService.mutateAsync({
        appSlug,
        kind: kindName,
        name: name.trim(),
        version,
        envVars,
      });
      queryClient.invalidateQueries({
        queryKey: ['apps', appSlug, 'services'],
      });
      onClose();
    } catch (e) {
      toast.error('Failed to provision service: ' + e);
    }
  };

  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Provision Service</DialogTitle>
        </DialogHeader>
        <VStack space={4} className="mt-4">
          <VStack space={1.5}>
            <label className="text-xs font-medium text-text-secondary">
              Service Name
            </label>
            <TextInput
              value={name}
              onChange={setName}
              placeholder="e.g. main-db"
            />
          </VStack>

          <HStack space={4}>
            <VStack space={1.5} className="flex-1">
              <label className="text-xs font-medium text-text-secondary">
                Type
              </label>
              <select
                value={kindName}
                onChange={(e) =>
                  handleKindChange(e.target.value as ServiceKind)
                }
                className="flex w-full cursor-pointer items-center rounded-lg border border-gray-300 px-3 py-[9px] text-sm focus-within:border-gray-400/90 outline-none bg-transparent"
              >
                {kinds.map((k) => (
                  <option key={k.name} value={k.name}>
                    {k.name}
                  </option>
                ))}
              </select>
            </VStack>

            <VStack space={1.5} className="w-1/3">
              <label className="text-xs font-medium text-text-secondary">
                Version
              </label>
              <select
                value={version}
                onChange={(e) => setVersion(e.target.value)}
                className="flex w-full cursor-pointer items-center rounded-lg border border-gray-300 px-3 py-[9px] text-sm focus-within:border-gray-400/90 outline-none bg-transparent"
              >
                {selectedKind?.supported_versions.map((v) => (
                  <option key={v} value={v}>
                    {v}
                  </option>
                ))}
              </select>
            </VStack>
          </HStack>

          <VStack space={1.5} className="mt-4">
            <label className="text-xs font-medium text-text-secondary">
              Environment Configuration
            </label>
            <div className="max-h-[400px] overflow-auto rounded-lg border border-border bg-surface/30 custom-scrollbar">
              <EnvEditor
                key={kindName}
                initialVars={envVars}
                isLoading={false}
                isSaving={false}
                onChange={setEnvVars}
                variant="embedded"
              />
            </div>
          </VStack>
        </VStack>
        <DialogFooter className="mt-6">
          <Button label="Cancel" variant="ghost" onClick={onClose} />
          <Button
            label="Provision"
            onClick={handleProvision}
            isLoading={provisionService.isPending}
            disabled={!name.trim() || !kindName || !version}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function ServiceConfigModal({
  appSlug,
  service,
  onClose,
}: {
  appSlug: string;
  service: Service;
  onClose: () => void;
}) {
  return (
    <Dialog open={true} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl border-none bg-transparent p-0 shadow-none">
        <ServiceEnvEditor
          appSlug={appSlug}
          serviceId={service.id}
          serviceName={service.name}
          readOnly={true}
          onCancel={onClose}
        />
      </DialogContent>
    </Dialog>
  );
}

function ServiceLogModal({
  serviceId,
  serviceName,
  appSlug,
  onClose,
}: {
  serviceId: string;
  serviceName: string;
  appSlug: string;
  onClose: () => void;
}) {
  const [logs, setLogs] = useState<string[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const token = getAuthToken();
    const url = `/api/apps/${appSlug}/services/${serviceId}/logs?token=${token}`;
    const es = new EventSource(url);

    es.onmessage = (event) => {
      const data = event.data;
      if (data && data !== '[done]') {
        setLogs((prev) => [...prev, data]);
      }
    };

    es.onerror = (e) => {
      console.error('SSE Stream error:', e);
    };

    return () => {
      es.close();
    };
  }, [appSlug, serviceId]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 px-4 py-8 backdrop-blur-sm">
      <div
        ref={containerRef}
        className="flex h-full w-full max-w-4xl flex-col rounded-lg border border-border bg-bg shadow-2xl"
      >
        <HStack
          justifyContent="between"
          className="shrink-0 border-b border-border p-4"
        >
          <HStack space={3}>
            <Terminal className="size-4 text-text-tertiary" />
            <h3 className="text-sm font-semibold text-text">
              Service Logs — {serviceName}
            </h3>
          </HStack>
          <Button label="Close" variant="ghost" size="sm" onClick={onClose} />
        </HStack>

        <div
          ref={scrollRef}
          className="flex-1 overflow-auto bg-black/40 p-6 font-mono text-[13px] leading-relaxed selection:bg-white/10"
        >
          {logs.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full gap-3 text-text-tertiary">
              <CircleDashed className="size-5 animate-spin" />
              <p>Establishing log stream...</p>
            </div>
          ) : (
            <div className="space-y-1">
              {logs.map((log, i) => (
                <div
                  key={i}
                  className="text-text-secondary whitespace-pre-wrap break-all"
                >
                  <span className="text-text-tertiary mr-3 select-none">
                    {i + 1}
                  </span>
                  {log}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
