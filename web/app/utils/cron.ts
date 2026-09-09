import cronstrue from 'cronstrue';

export function describeSchedule(expression: string | null | undefined) {
  if (!expression?.trim()) {
    return null;
  }

  try {
    return cronstrue.toString(expression, { verbose: false });
  } catch {
    return null;
  }
}
