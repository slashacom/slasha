type Option = {
  id: string;
  value: string;
};

type ButtonSwitchProps = {
  options: Option[];
  selectedOption: string;
  onSelect: (optionId: string) => void;
};

export function ButtonSwitch(props: ButtonSwitchProps) {
  const { options, selectedOption, onSelect } = props;
  return (
    <div className="flex items-center border border-neutral-200 overflow-hidden rounded-lg text-xs text-neutral-400">
      {options.map((option) => (
        <button
          key={option.id}
          onClick={() => onSelect(option.id)}
          className={`text-xs px-2 py-1 cursor-pointer ${
            selectedOption === option.id ? 'text-black bg-neutral-200' : ''
          }`}
        >
          {option.value}
        </button>
      ))}
    </div>
  );
}
