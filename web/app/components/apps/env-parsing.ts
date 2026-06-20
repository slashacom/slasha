export type EnvVar = { key: string; value: string };

export const fromEnvRecord = (
  record: Record<string, string> | undefined
): EnvVar[] => {
  return Object.entries(record ?? {}).map(([key, value]) => ({ key, value }));
};

export const toEnvRecord = (vars: EnvVar[]): Record<string, string> => {
  const record: Record<string, string> = {};
  vars.forEach((v) => {
    if (v.key.trim()) {
      record[v.key.trim()] = v.value;
    }
  });
  return record;
};

export function parseDotEnv(text: string): EnvVar[] {
  const out: EnvVar[] = [];
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) {
      continue;
    }
    const eq = line.indexOf('=');
    if (eq === -1) {
      continue;
    }
    const key = line.slice(0, eq).trim();
    if (!key) {
      continue;
    }
    let value = line.slice(eq + 1).trim();
    if (
      value.length >= 2 &&
      ((value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'")))
    ) {
      value = value.slice(1, -1);
    }
    out.push({ key, value });
  }
  return out;
}

export function serializeDotEnv(vars: EnvVar[]): string {
  return vars
    .filter((v) => v.key.trim())
    .map((v) => {
      const value = v.value;
      const needsQuoting =
        /[\s#"']/.test(value) || value.includes('\n') || value === '';
      if (!needsQuoting) {
        return `${v.key.trim()}=${value}`;
      }
      const escaped = value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
      return `${v.key.trim()}="${escaped}"`;
    })
    .join('\n');
}

export function looksLikeDotEnv(text: string): boolean {
  if (!text.includes('\n') && !text.includes('=')) {
    return false;
  }
  const lines = text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l && !l.startsWith('#'));
  if (lines.length === 0) {
    return false;
  }
  const withEq = lines.filter((l) => l.includes('=')).length;
  return withEq / lines.length >= 0.6;
}

export const noAutofillProps = {
  autoComplete: 'off',
  autoCorrect: 'off',
  autoCapitalize: 'off',
  spellCheck: false,
  'data-1p-ignore': 'true',
  'data-lpignore': 'true',
  'data-form-type': 'other',
} as const;
