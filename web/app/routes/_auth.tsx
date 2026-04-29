import { Navigate, Outlet } from 'react-router';
import { isLoggedIn } from '~/utils/jwt';

export default function Auth() {
  if (isLoggedIn()) {
    return <Navigate to="/apps" replace />;
  }

  return (
    <div className="flex min-h-screen w-full items-center justify-center bg-bg">
      <div className="w-full max-w-[400px] px-6">
        <Outlet />
      </div>
    </div>
  );
}
