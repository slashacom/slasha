import { redirect } from 'react-router';
import type { Route } from './+types/_index';
import { queryClient } from '~/utils/query-client';
import { getAuthMeOptions } from '~/queries/auth';

export function meta({}: Route.MetaArgs) {
  return [
    { title: 'slasha' },
    {
      name: 'description',
      content: 'A self hostable, open source PaaS for developers.',
    },
  ];
}

export default function Index() {
  return (
    <div className="flex h-screen items-center justify-center">
      <div className="relative flex flex-col items-center justify-center gap-2">
        <h1 className="text-6xl font-bold text-black">Slasha</h1>
        <p className="max-w-2xl py-2 text-center text-2xl text-gray-500">
          A self hostable, open source PaaS for developers.
        </p>
      </div>
    </div>
  );
}
