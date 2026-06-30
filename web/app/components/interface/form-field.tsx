import { Label } from '~/components/interface/label';

type FormFieldProps = {
  label: string;
  help?: string;
  children: React.ReactNode;
};

export function FormField(props: FormFieldProps) {
  const { label, help, children } = props;

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <Label>{label}</Label>
        {help ? (
          <span className="text-[11px] text-text-tertiary">{help}</span>
        ) : null}
      </div>
      {children}
    </div>
  );
}
