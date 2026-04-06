import { Outlet } from 'react-router';
import { Sidebar } from '~/components/global/sidebar';

export default function UserLayout() {
  return (
    <div className="flex min-h-screen w-full bg-white">
      <Sidebar />
      <main className="flex-grow">
        <Outlet />
      </main>
    </div>
  );
}
