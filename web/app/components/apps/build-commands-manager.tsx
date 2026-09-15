import { useState } from 'react';
import { useQueryClient, useSuspenseQuery } from '@tanstack/react-query';
import { Terminal } from 'lucide-react';
import { toast } from 'sonner';
import { getAppEnvVarsOptions, useUpdateAppEnvVars } from '~/queries/apps';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { HStack, VStack } from '~/components/interface/stacks';
import { SettingsCard } from '~/components/interface/settings-card';

const BUILD_CMD_KEY = 'SLASHA_BUILD_CMD';
const START_CMD_KEY = 'SLASHA_START_CMD';

type BuildCommandsManagerProps = {
  appSlug: string;
};

export function BuildCommandsManager(props: BuildCommandsManagerProps) {
  const { appSlug } = props;
  const queryClient = useQueryClient();
  const updateEnvVars = useUpdateAppEnvVars();
  const { data: envData } = useSuspenseQuery(getAppEnvVarsOptions(appSlug));

  const savedBuild = envData.env_vars[BUILD_CMD_KEY] ?? '';
  const savedStart = envData.env_vars[START_CMD_KEY] ?? '';

  const [buildCmd, setBuildCmd] = useState(savedBuild);
  const [startCmd, setStartCmd] = useState(savedStart);

  const trimmedBuild = buildCmd.trim();
  const trimmedStart = startCmd.trim();
  const isDirty = trimmedBuild !== savedBuild || trimmedStart !== savedStart;

  const handleSave = async () => {
    if (!isDirty) {
      return;
    }

    const vars = { ...envData.env_vars };

    if (trimmedBuild) {
      vars[BUILD_CMD_KEY] = trimmedBuild;
    } else {
      delete vars[BUILD_CMD_KEY];
    }

    if (trimmedStart) {
      vars[START_CMD_KEY] = trimmedStart;
    } else {
      delete vars[START_CMD_KEY];
    }

    const promise = updateEnvVars.mutateAsync({ appSlug, vars });

    toast.promise(promise, {
      loading: 'Saving build settings...',
      success: () => {
        queryClient.invalidateQueries({
          queryKey: ['apps', appSlug, 'env-vars'],
        });
        setBuildCmd(trimmedBuild);
        setStartCmd(trimmedStart);
        return 'Build settings saved';
      },
      error: (error) => error.message || 'Failed to save build settings.',
    });
  };

  return (
    <SettingsCard
      icon={Terminal}
      title="Build & Start"
      description="Override how the app is built and started. Leave blank to let Slasha detect them from the repository."
    >
      <VStack space={3}>
        <VStack space={1.5}>
          <Label
            htmlFor="build-command"
            className="text-[12px] font-medium text-text-secondary"
          >
            Build command
          </Label>
          <Input
            id="build-command"
            value={buildCmd}
            onChange={(event) => setBuildCmd(event.target.value)}
            placeholder="npm run build"
            className="max-w-xl font-mono"
          />
        </VStack>
        <VStack space={1.5}>
          <Label
            htmlFor="start-command"
            className="text-[12px] font-medium text-text-secondary"
          >
            Start command
          </Label>
          <Input
            id="start-command"
            value={startCmd}
            onChange={(event) => setStartCmd(event.target.value)}
            placeholder="npm start"
            className="max-w-xl font-mono"
          />
        </VStack>
        <HStack>
          <Button
            label="Save"
            onClick={handleSave}
            disabled={updateEnvVars.isPending || !isDirty}
          />
        </HStack>
      </VStack>

      <p className="mt-3 max-w-prose text-[12px] text-text-tertiary">
        The build command only applies to Railpack builds. The start command
        runs as the <span className="font-mono text-text-secondary">web</span>{' '}
        process and takes precedence over a Procfile or the detected start
        script. Changes apply from the next deployment.
      </p>
    </SettingsCard>
  );
}
