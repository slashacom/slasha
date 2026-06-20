import { cn } from '~/utils/classname';

function Skeleton(componentProps: React.ComponentProps<'div'>) {
  const { className, ...props } = componentProps;
  return (
    <div
      data-slot="skeleton"
      className={cn('animate-pulse bg-white/5 rounded-md', className)}
      {...props}
    />
  );
}

export { Skeleton };
