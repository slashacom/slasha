import { FormField } from '~/components/interface/form-field';
import { Input } from '~/components/interface/input';

type NumberFieldProps = {
  label: string;
  value: string;
  min: number;
  max?: number;
  step: number;
  onChange: (value: string) => void;
};

export function NumberField(props: NumberFieldProps) {
  const { label, value, min, max, step, onChange } = props;

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
