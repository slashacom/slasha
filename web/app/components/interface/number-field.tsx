import { FormField } from '~/components/interface/form-field';
import { Input } from '~/components/interface/input';

type NumberFieldProps = {
  label: string;
  value: string;
  min: number;
  max?: number;
  step: number;
  suffix?: string;
  onChange: (value: string) => void;
};

export function NumberField(props: NumberFieldProps) {
  const { label, value, min, max, step, suffix, onChange } = props;

  if (!suffix) {
    return (
      <FormField label={label}>
        <Input
          type="number"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(event) => onChange(event.target.value)}
        />
      </FormField>
    );
  }

  return (
    <FormField label={label}>
      <div className="relative">
        <Input
          type="number"
          min={min}
          max={max}
          step={step}
          value={value}
          className="pr-10"
          onChange={(event) => onChange(event.target.value)}
        />
        <span className="pointer-events-none absolute inset-y-0 right-3 flex items-center text-[13px] text-text-tertiary">
          {suffix}
        </span>
      </div>
    </FormField>
  );
}
