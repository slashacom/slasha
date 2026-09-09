import { ArrowLeft } from 'lucide-react';
import { Link } from 'react-router';

type BackButtonProps = {
  to: string;
  label?: string;
};

export function BackButton(props: BackButtonProps) {
  const { to, label = 'Go back' } = props;

  return (
    <Link
      to={to}
      aria-label={label}
      title={label}
      className="group flex size-7 shrink-0 items-center justify-center rounded border border-border bg-surface !no-underline transition-colors hover:bg-white/[0.06]"
    >
      <ArrowLeft className="size-3.5 text-text-tertiary transition-colors group-hover:text-text" />
    </Link>
  );
}
