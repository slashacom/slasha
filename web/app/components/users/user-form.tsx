import { useState } from 'react';
import { SearchIcon } from 'lucide-react';
import { Button } from '~/components/interface/button';
import { Input } from '~/components/interface/input';
import { Label } from '~/components/interface/label';
import { Select } from '~/components/interface/select';
import { Checkbox } from '~/components/interface/checkbox';
import type { User, UserRole } from '~/models/user';
import type { App } from '~/models/app';

type UserFormProps = {
  initialData?: User;
  initialAppIds?: string[];
  apps?: App[];
  onSubmit: (e: React.SubmitEvent<HTMLFormElement>) => void;
  onCancel: () => void;
  isPending: boolean;
  submitLabel: string;
};

export function UserForm(props: UserFormProps) {
  const {
    initialData,
    initialAppIds,
    apps,
    onSubmit,
    onCancel,
    isPending,
    submitLabel,
  } = props;

  const [role, setRole] = useState<UserRole>(initialData?.role || 'User');
  const [selectedAppIds, setSelectedAppIds] = useState<string[]>(
    initialAppIds || []
  );
  const [searchQuery, setSearchQuery] = useState('');

  const handleToggleApp = (appId: string) => {
    setSelectedAppIds((prev) =>
      prev.includes(appId)
        ? prev.filter((id) => id !== appId)
        : [...prev, appId]
    );
  };

  const filteredApps =
    apps?.filter(
      (app) =>
        app.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        app.slug.toLowerCase().includes(searchQuery.toLowerCase())
    ) || [];

  return (
    <form onSubmit={onSubmit} className="w-full max-w-md">
      <div className="space-y-5">
        <div className="space-y-1.5">
          <Label
            htmlFor="email"
            className="text-[13px] font-medium text-text-secondary"
          >
            Email address
          </Label>
          <Input
            id="email"
            name="email"
            type="email"
            required
            defaultValue={initialData?.email}
            placeholder="user@example.com"
            className="h-10"
          />
        </div>

        <div className="space-y-1.5">
          <Label
            htmlFor="password"
            className="text-[13px] font-medium text-text-secondary"
          >
            {initialData ? 'New Password (optional)' : 'Password'}
          </Label>
          <Input
            id="password"
            name="password"
            type="password"
            required={!initialData}
            pattern=".{8,}"
            title="8 characters minimum"
            placeholder={
              initialData
                ? 'Leave blank to keep unchanged'
                : 'At least 8 characters'
            }
            className="h-10"
          />
        </div>

        <div className="space-y-1.5">
          <Label
            htmlFor="role"
            className="text-[13px] font-medium text-text-secondary"
          >
            Role
          </Label>
          <Select
            id="role"
            name="role"
            required
            defaultValue={initialData?.role || 'User'}
            onChange={(e) => setRole(e.target.value as UserRole)}
          >
            <option value="User">User</option>
            <option value="Admin">Admin</option>
          </Select>
        </div>

        {role === 'User' && apps && (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-[13px] font-medium text-text-secondary">
                App Memberships
              </Label>
              {apps.length > 0 && (
                <div className="flex items-center gap-3">
                  <button
                    type="button"
                    onClick={() => {
                      const visibleIds = filteredApps.map((a) => a.id);
                      setSelectedAppIds((prev) => {
                        const next = [...prev];
                        for (const id of visibleIds) {
                          if (!next.includes(id)) next.push(id);
                        }
                        return next;
                      });
                    }}
                    className="text-xs text-text-secondary hover:text-text cursor-pointer select-none transition-colors"
                  >
                    Select all
                  </button>
                  <span className="text-text-tertiary text-xs">|</span>
                  <button
                    type="button"
                    onClick={() => {
                      const visibleIds = filteredApps.map((a) => a.id);
                      setSelectedAppIds((prev) =>
                        prev.filter((id) => !visibleIds.includes(id))
                      );
                    }}
                    className="text-xs text-text-secondary hover:text-text cursor-pointer select-none transition-colors"
                  >
                    Clear all
                  </button>
                </div>
              )}
            </div>

            {apps.length > 0 && (
              <div className="relative">
                <SearchIcon className="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-text-tertiary" />
                <Input
                  type="text"
                  placeholder="Search apps..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="h-9 pl-9 pr-4 text-sm border-border bg-surface placeholder:text-text-tertiary focus-visible:border-text-secondary focus-visible:ring-0"
                />
              </div>
            )}

            <div className="border border-border rounded-md bg-surface p-4 max-h-56 overflow-y-auto space-y-3 custom-scrollbar">
              {apps.length === 0 ? (
                <span className="text-sm text-text-secondary">
                  No apps created yet.
                </span>
              ) : filteredApps.length === 0 ? (
                <span className="text-sm text-text-secondary">
                  No matching apps found.
                </span>
              ) : (
                filteredApps.map((app) => (
                  <div key={app.id} className="flex items-center space-x-3">
                    <Checkbox
                      id={`app-${app.id}`}
                      checked={selectedAppIds.includes(app.id)}
                      onCheckedChange={() => handleToggleApp(app.id)}
                    />
                    <Label
                      htmlFor={`app-${app.id}`}
                      className="text-sm font-normal text-text cursor-pointer select-none"
                    >
                      {app.name}
                    </Label>
                  </div>
                ))
              )}
            </div>

            {/* Hidden inputs to pass data via FormData */}
            {selectedAppIds.map((id) => (
              <input type="hidden" key={id} name="app_ids" value={id} />
            ))}
          </div>
        )}

        <div className="flex items-center justify-end gap-2 pt-2">
          <Button
            variant="ghost"
            label="Cancel"
            onClick={onCancel}
            isDisabled={isPending}
          />
          <Button
            type="submit"
            label={submitLabel}
            isLoading={isPending}
            isDisabled={isPending}
          />
        </div>
      </div>
    </form>
  );
}
