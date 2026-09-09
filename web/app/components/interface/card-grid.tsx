import { cn } from '~/utils/classname';

type CardGridProps = {
  children: React.ReactNode;
  className?: string;
};

export function CardGrid(props: CardGridProps) {
  const { children, className } = props;

  return (
    <div
      className={cn(
        'grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(250px,1fr))]',
        className
      )}
    >
      {children}
    </div>
  );
}
