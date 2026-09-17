import { useState } from 'react';
import { Lock, Globe, Eye, EyeOff } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import type { App, AppVisibility } from '~/models/app';
import { useUpdateAppVisibility } from '~/queries/apps';
import { SettingsCard } from '~/components/interface/settings-card';
import { Button } from '~/components/interface/button';
import { PasswordInput } from '~/components/interface/password-input';
import { Label } from '~/components/interface/label';
import { cn } from '~/utils/classname';

type AppVisibilityManagerProps = {
  app: App;
};

const VISIBILITY_OPTIONS: {
  value: AppVisibility;
  label: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
}[] = [
  {
    value: 'public',
    label: 'Public',
    description: 'Anyone on the internet can access this app.',
    icon: Globe,
  },
  {
    value: 'password',
    label: 'Password Protected',
    description: 'Visitors must enter a password to access this app.',
    icon: Lock,
  },
  {
    value: 'private',
    label: 'Private',
    description: 'Only members of this application can access it.',
    icon: Eye,
  },
];

export function AppVisibilityManager(props: AppVisibilityManagerProps) {
  const { app } = props;
  const queryClient = useQueryClient();
  const updateVisibility = useUpdateAppVisibility();

  const [selected, setSelected] = useState<AppVisibility>(app.visibility);
  const [password, setPassword] = useState('');

  const isPasswordRequired =
    selected === 'password' && app.visibility !== 'password';
  const hasChanges =
    selected !== app.visibility ||
    (selected === 'password' && password.length > 0);
  const isValid = !isPasswordRequired || password.length > 0;

  const handleSave = async () => {
    try {
      await updateVisibility.mutateAsync({
        appSlug: app.slug,
        visibility: selected,
        password: selected === 'password' && password ? password : undefined,
      });
      toast.success('App visibility updated successfully');
      setPassword('');
      queryClient.invalidateQueries({ queryKey: ['apps', app.slug] });
    } catch (e: any) {
      toast.error(e?.message || 'Failed to update visibility');
    }
  };

  const body = (
    <div className="p-6 space-y-5">
      <div className="space-y-2">
        {VISIBILITY_OPTIONS.map((opt) => {
          const Icon = opt.icon;
          const isSelected = selected === opt.value;
          return (
            <button
              key={opt.value}
              type="button"
              onClick={() => setSelected(opt.value)}
              className={cn(
                'w-full flex items-start gap-3.5 rounded-lg border px-4 py-3.5 text-left transition-all',
                isSelected
                  ? 'border-border bg-white/[0.06] text-text shadow-sm'
                  : 'border-border/60 bg-surface/20 text-text-secondary hover:bg-surface/50 hover:border-border'
              )}
            >
              <div
                className={cn(
                  'mt-0.5 shrink-0 rounded-md p-1.5 transition-colors',
                  isSelected
                    ? 'bg-white/10 text-text'
                    : 'bg-white/5 text-text-tertiary'
                )}
              >
                <Icon className="size-4" />
              </div>
              <div className="min-w-0">
                <p
                  className={cn(
                    'text-[13px] font-medium',
                    isSelected ? 'text-text' : 'text-text-secondary'
                  )}
                >
                  {opt.label}
                </p>
                <p className="mt-0.5 text-[12px] text-text-tertiary">
                  {opt.description}
                </p>
              </div>
              <div className="ml-auto mt-1 shrink-0">
                <div
                  className={cn(
                    'size-4 rounded-full border-2 transition-colors flex items-center justify-center',
                    isSelected
                      ? 'border-text bg-text'
                      : 'border-border bg-transparent'
                  )}
                >
                  {isSelected && (
                    <div className="size-1.5 rounded-full bg-bg" />
                  )}
                </div>
              </div>
            </button>
          );
        })}
      </div>

      {selected === 'password' && (
        <div className="space-y-2">
          <Label className="text-[13px] font-medium text-text-secondary">
            {app.visibility === 'password' ? 'Change Password' : 'Set Password'}
          </Label>
          <PasswordInput
            placeholder={
              app.visibility === 'password'
                ? 'Leave blank to keep current password…'
                : 'Enter access password…'
            }
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="text-[13px]"
          />
          {app.visibility === 'password' && (
            <p className="text-[11px] text-text-tertiary">
              A password is currently active. Enter a new password to update it.
            </p>
          )}
        </div>
      )}

      <Button
        label="Save Visibility"
        onClick={handleSave}
        isLoading={updateVisibility.isPending}
        isDisabled={!hasChanges || !isValid || updateVisibility.isPending}
        size="sm"
      />
    </div>
  );

  return (
    <SettingsCard
      icon={Lock}
      title="App Visibility"
      description="Control access permissions for this application."
      body={body}
    />
  );
}
