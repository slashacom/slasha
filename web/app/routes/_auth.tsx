import { Outlet } from 'react-router';

export default function Auth() {
  return (
    <div className="flex min-h-screen w-full flex-col items-center justify-center bg-white paper-lines">
      <div className="relative w-full max-w-[420px] px-6 text-center">
        <Outlet />
      </div>
    </div>
  );
}
