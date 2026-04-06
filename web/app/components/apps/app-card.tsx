import { Link } from 'react-router';
import { VStack, HStack } from '~/components/interface/stacks';
import { cn } from '~/utils/classname';
import type { App } from '~/models/app';
import { GitBranchIcon, CalendarIcon } from 'lucide-react';

export function AppCard({ app }: { app: App }) {
  return (
    <Link
      to={`/apps/${app.slug}`}
      className="group block overflow-hidden rounded-xl border border-neutral-200 bg-white p-5 transition-all"
    >
      <VStack space={4}>
        <HStack justifyContent="between" alignItems="start">
          <VStack space={1}>
            <h3 className="text-lg font-semibold text-neutral-900">
              {app.name}
            </h3>
            <p className="text-sm text-neutral-500">{app.slug}</p>
          </VStack>
          <div
            className={cn(
              'rounded-full px-2 py-0.5 text-xs font-medium',
              app.status === 'active'
                ? 'bg-green-100 text-green-700'
                : 'bg-neutral-100 text-neutral-600'
            )}
          >
            {app.status}
          </div>
        </HStack>

        <HStack space={4} className="text-xs text-neutral-400">
          <HStack space={1.5}>
            <GitBranchIcon className="size-3.5" />
            <span>{app.default_branch}</span>
          </HStack>
          <HStack space={1.5}>
            <CalendarIcon className="size-3.5" />
            <span>{new Date(app.created_at).toLocaleDateString()}</span>
          </HStack>
        </HStack>
      </VStack>
    </Link>
  );
}
