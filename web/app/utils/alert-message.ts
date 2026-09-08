import { emojify } from 'node-emoji';

export type AlertMessageField = {
  label: string;
  value: string;
};

export type ParsedAlertMessage = {
  title: string;
  fields: AlertMessageField[];
  body: string[];
};

const QUOTE_PREFIX = /^>\s*/;
const BOLD = /\*([^*]+)\*/g;
const FIELD = /^\*([^*]+)\*\s*:\s*(.+)$/;
const BOLD_FIELD = /^\*([^*]+):\*\s*(.+)$/;

function stripBold(value: string) {
  return value.replace(BOLD, '$1');
}

export function parseAlertMessage(message: string): ParsedAlertMessage {
  const lines = emojify(message)
    .split('\n')
    .map((line) => line.replace(QUOTE_PREFIX, '').trim())
    .filter(Boolean);

  const fields: AlertMessageField[] = [];
  const body: string[] = [];
  let title = '';

  for (const line of lines) {
    const field = line.match(FIELD) ?? line.match(BOLD_FIELD);
    if (field) {
      fields.push({ label: field[1].trim(), value: stripBold(field[2]) });
      continue;
    }

    if (!title) {
      title = stripBold(line);
      continue;
    }

    body.push(stripBold(line));
  }

  return { title, fields, body };
}
