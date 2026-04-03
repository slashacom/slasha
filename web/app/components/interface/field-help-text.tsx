import { cn } from '~/utils/classname';

type FieldHelpTextProps = {
  text: string;
  className?: string;
};

export function FieldHelpText(props: FieldHelpTextProps) {
  const { text, className = '' } = props;
  return (
    <p className={cn('text-xs mt-1 leading-5 text-gray-400', className)}>
      {text}
    </p>
  );
}
