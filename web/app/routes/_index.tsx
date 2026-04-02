import { Link } from 'react-router';
import type { Route } from './+types/_index';

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

        <div className="flex flex-col items-center justify-center gap-4">
          <div className="flex flex-row items-center justify-center gap-2 my-4">
            <Link
              to="/login"
              className="flex flex-row items-center gap-2 rounded-full bg-black px-8 py-2 text-white hover:opacity-80"
            >
              Sign up
            </Link>
          </div>
        </div>
      </div>
    </div>
  );
}
