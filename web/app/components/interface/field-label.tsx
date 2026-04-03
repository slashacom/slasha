import { cn } from '~/utils/classname';

type FieldLabelProps = {
  label: string;
  className?: string;
  htmlFor?: string;
};

export function FieldLabel(props: FieldLabelProps) {
  const { label, className = '', htmlFor } = props;

  return (
    <label
      className={cn('mb-1.5 block text-gray-500 text-sm', className)}
      htmlFor={htmlFor}
    >
      {label}
    </label>
  );
}
