import { Spinner } from '~/components/icons/spinner';

export function FullPageSpinner() {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-bg">
      <Spinner className="h-5 w-5" />
    </div>
  );
}
