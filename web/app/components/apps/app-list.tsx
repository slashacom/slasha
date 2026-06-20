import { AppCard } from './app-card';
import type { App } from '~/models/app';

type AppListProps = {
  apps: App[];
};

export function AppList(props: AppListProps) {
  const { apps } = props;
  if (apps.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-border bg-surface/40 px-6 py-16 text-center">
        <p className="text-sm text-text-secondary">
          No apps yet. Create your first app to get started.
        </p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
      {apps.map((app) => (
        <AppCard key={app.id} app={app} />
      ))}
    </div>
  );
}
