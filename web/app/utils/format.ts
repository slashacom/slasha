const EXT_TO_LANG: Record<string, string> = {
  rs: 'rust',
  ts: 'typescript',
  tsx: 'tsx',
  js: 'javascript',
  jsx: 'jsx',
  py: 'python',
  rb: 'ruby',
  go: 'go',
  java: 'java',
  kt: 'kotlin',
  swift: 'swift',
  c: 'c',
  cpp: 'cpp',
  h: 'c',
  hpp: 'cpp',
  css: 'css',
  scss: 'scss',
  html: 'html',
  json: 'json',
  yaml: 'yaml',
  yml: 'yaml',
  toml: 'toml',
  md: 'markdown',
  sql: 'sql',
  sh: 'bash',
  bash: 'bash',
  zsh: 'bash',
  dockerfile: 'dockerfile',
  xml: 'xml',
  svg: 'xml',
  vue: 'vue',
  php: 'php',
  lua: 'lua',
  zig: 'zig',
  diff: 'diff',
  graphql: 'graphql',
  prisma: 'prisma',
  makefile: 'makefile',
};

export function inferLang(filename: string): string {
  const lower = filename.toLowerCase();
  if (lower === 'dockerfile' || lower.startsWith('dockerfile.')) {
    return 'dockerfile';
  }
  if (lower === 'makefile' || lower === 'gnumakefile') {
    return 'makefile';
  }

  const ext = filename.split('.').pop()?.toLowerCase() ?? '';
  return EXT_TO_LANG[ext] ?? 'text';
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  if (bytes < 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function parseUTC(str: string): Date {
  if (!str.endsWith('Z') && !str.includes('+')) {
    return new Date(str + 'Z');
  }
  return new Date(str);
}

export function formatRelativeTime(date: string | Date): string {
  const now = new Date();
  const then = typeof date === 'string' ? parseUTC(date) : date;
  const seconds = Math.floor((now.getTime() - then.getTime()) / 1000);

  if (seconds < 60) {
    return 'just now';
  }
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return `${minutes}m ago`;
  }
  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours}h ago`;
  }
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}
