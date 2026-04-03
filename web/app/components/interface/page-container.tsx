import { cn } from '~/utils/classname';

export type PageContainerProps = {
  children: React.ReactNode;
  className?: string;
  variant?: 'center' | 'full';
};

export function PageContainer(props: PageContainerProps) {
  const { children, className, variant = 'narrow' } = props;

  return (
    <div
      className={cn(
        'relative mx-auto flex h-full w-full flex-grow flex-col',
        {
          'max-w-7xl': variant === 'center',
          'w-full': variant === 'full',
        },
        className
      )}
    >
      {children}
    </div>
  );
}
