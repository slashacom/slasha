import { Outlet } from 'react-router';

export default function Auth() {
  return (
    <div className="flex min-h-screen w-full items-center justify-center bg-bg">
      <div className="w-full max-w-[400px] px-6">
        <Outlet />
      </div>
    </div>
  );
}
