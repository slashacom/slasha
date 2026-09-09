import { cn } from '~/utils/classname';

type PageProps = {
  children: React.ReactNode;
  className?: string;
};

export function Page(props: PageProps) {
  const { children, className } = props;

  return <div className={cn('px-8 py-6', className)}>{children}</div>;
}
