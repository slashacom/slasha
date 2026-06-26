import { HStack } from '~/components/interface/stacks';

type EnvVarChipsProps = {
  keys: string[];
};

export function EnvVarChips(props: EnvVarChipsProps) {
  const { keys } = props;
  return (
    <HStack space={1.5} wrap>
      {keys.map((key) => (
        <span
          key={key}
          className="rounded bg-white/5 px-1.5 py-0.5 font-mono text-[11px] text-text-secondary"
        >
          {key}
        </span>
      ))}
    </HStack>
  );
}
