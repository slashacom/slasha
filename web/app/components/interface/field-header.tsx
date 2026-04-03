import { cn } from '~/utils/classname';

type FieldHeaderProps = {
  label: string;
  description?: string;
  className?: string;
};

export function FieldHeader(props: FieldHeaderProps) {
  const { label, description, className = '' } = props;

  return (
    <div className={cn('mb-3', className)}>
      <h1 className="text-xl font-semibold">{label}</h1>
      {description && (
        <p className="mt-1.5 text-sm leading-5 text-gray-500">{description}</p>
      )}
    </div>
  );
}
