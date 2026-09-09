import { Boxes, Plus } from 'lucide-react';
import { AppCard } from './app-card';
import { EmptyPage } from '~/components/global/empty-page';
import { CardGrid } from '~/components/interface/card-grid';
import type { AppListItem } from '~/queries/apps';

type AppListProps = {
  apps: AppListItem[];
};

export function AppList(props: AppListProps) {
  const { apps } = props;
  if (apps.length === 0) {
    return (
      <EmptyPage
        icon={Boxes}
        size="lg"
        title="No apps yet."
        subtitle="An app is a deployable service with its own repository, domains, and environment. Create one to get started."
        actionLabel="Create app"
        actionIcon={<Plus className="size-3.5" />}
        actionTo="/apps/new"
      />
    );
  }

  return (
    <CardGrid>
      {apps.map((item) => (
        <AppCard key={item.app.id} item={item} />
      ))}
    </CardGrid>
  );
}
