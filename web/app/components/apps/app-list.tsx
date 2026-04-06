import { AppCard } from './app-card';
import type { App } from '~/models/app';

export function AppList({ apps }: { apps: App[] }) {
  if (apps.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-xl border-2 border-dashed border-neutral-200 py-20">
        <p className="text-neutral-500">
          No apps found. Create your first app to get started!
        </p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
      {apps.map((app) => (
        <AppCard key={app.id} app={app} />
      ))}
    </div>
  );
}
