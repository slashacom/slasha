import { useState, useMemo, useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ChevronDown, ChevronRight } from 'lucide-react';
import type { ServiceKind } from '~/models/service';
import {
  getServiceKindsOptions,
  useProvisionService,
} from '~/queries/services';
import { Button } from '~/components/interface/button';
import { HStack, VStack } from '~/components/interface/stacks';
import { toast } from 'sonner';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '~/components/interface/dialog';
import { TextInput } from '~/components/interface/text-input';
import { EnvEditor } from '~/components/apps/env-editor';
import { buildResourcesPayload } from '~/components/apps/service-resources';

export function ProvisionServiceModal(props: {
  appSlug: string;
  onClose: () => void;
}) {
  const { appSlug, onClose } = props;
  const queryClient = useQueryClient();
  const { data } = useQuery(getServiceKindsOptions());
  const provisionService = useProvisionService();

  const kinds = data?.kinds ?? [];

  const [name, setName] = useState('');
  const [kindName, setKindName] = useState<ServiceKind | ''>('');
  const [version, setVersion] = useState<string>('');
  const [envVars, setEnvVars] = useState<Record<string, string>>({});

  const [isAdvancedOpen, setIsAdvancedOpen] = useState(false);
  const [memoryMb, setMemoryMb] = useState('');
  const [cpuCores, setCpuCores] = useState('');
  const [shmMb, setShmMb] = useState('');
  const [pidsLimit, setPidsLimit] = useState('');

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

    const { payload, error } = buildResourcesPayload(
      memoryMb,
      cpuCores,
      shmMb,
      pidsLimit
    );
    if (error) {
      toast.error(error);
      return;
    }

    try {
      await provisionService.mutateAsync({
        appSlug,
        kind: kindName,
        name: name.trim(),
        version,
        envVars,
        resources: payload,
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

          <AdvancedResourcesSection
            isOpen={isAdvancedOpen}
            onToggle={() => setIsAdvancedOpen((v) => !v)}
            memoryMb={memoryMb}
            cpuCores={cpuCores}
            shmMb={shmMb}
            pidsLimit={pidsLimit}
            onMemoryChange={setMemoryMb}
            onCpuChange={setCpuCores}
            onShmChange={setShmMb}
            onPidsChange={setPidsLimit}
          />
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

function AdvancedResourcesSection(props: {
  isOpen: boolean;
  onToggle: () => void;
  memoryMb: string;
  cpuCores: string;
  shmMb: string;
  pidsLimit: string;
  onMemoryChange: (v: string) => void;
  onCpuChange: (v: string) => void;
  onShmChange: (v: string) => void;
  onPidsChange: (v: string) => void;
}) {
  const {
    isOpen,
    onToggle,
    memoryMb,
    cpuCores,
    shmMb,
    pidsLimit,
    onMemoryChange,
    onCpuChange,
    onShmChange,
    onPidsChange,
  } = props;
  return (
    <VStack space={2} className="mt-4">
      <button
        type="button"
        onClick={onToggle}
        className="flex items-center gap-1.5 text-xs font-medium text-text-secondary hover:text-text transition-colors w-fit"
      >
        {isOpen ? (
          <ChevronDown className="size-3.5" />
        ) : (
          <ChevronRight className="size-3.5" />
        )}
        Advanced
      </button>

      {isOpen && (
        <VStack
          space={3}
          className="rounded-lg border border-border bg-surface/30 p-4"
        >
          <p className="text-[11px] text-text-tertiary">
            Per-container resource caps. Leave blank for unlimited.
          </p>

          <HStack space={3}>
            <VStack space={1.5} className="flex-1">
              <label className="text-[11px] font-medium text-text-secondary">
                Memory (MB)
              </label>
              <TextInput
                value={memoryMb}
                onChange={onMemoryChange}
                placeholder="unlimited"
              />
            </VStack>
            <VStack space={1.5} className="flex-1">
              <label className="text-[11px] font-medium text-text-secondary">
                CPU (cores)
              </label>
              <TextInput
                value={cpuCores}
                onChange={onCpuChange}
                placeholder="unlimited"
              />
            </VStack>
          </HStack>

          <HStack space={3}>
            <VStack space={1.5} className="flex-1">
              <label className="text-[11px] font-medium text-text-secondary">
                Shared Memory (MB)
              </label>
              <TextInput
                value={shmMb}
                onChange={onShmChange}
                placeholder="unlimited"
              />
            </VStack>
            <VStack space={1.5} className="flex-1">
              <label className="text-[11px] font-medium text-text-secondary">
                PID Limit
              </label>
              <TextInput
                value={pidsLimit}
                onChange={onPidsChange}
                placeholder="unlimited"
              />
            </VStack>
          </HStack>
        </VStack>
      )}
    </VStack>
  );
}
