import { cn } from '~/utils/classname';

function Skeleton(componentProps: React.ComponentProps<'div'>) {
  const { className, ...props } = componentProps;
  return (
    <div
      data-slot="skeleton"
      className={cn('bg-accent animate-pulse rounded-md', className)}
      {...props}
    />
  );
}

export { Skeleton };
