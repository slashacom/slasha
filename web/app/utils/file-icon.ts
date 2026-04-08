import {
  FileText,
  FileCode,
  FileJson,
  FileImage,
  FileType,
  FileLock,
  FileTerminal,
  FileCog,
  FileArchive,
  Braces,
  Hash,
  type LucideIcon,
} from 'lucide-react';

const NAME_MAP: Record<string, LucideIcon> = {
  dockerfile: FileTerminal,
  makefile: FileTerminal,
  gnumakefile: FileTerminal,
  '.gitignore': FileCog,
  '.gitattributes': FileCog,
  '.env': FileLock,
  '.env.local': FileLock,
  '.env.example': FileLock,
  'cargo.toml': FileCog,
  'cargo.lock': FileLock,
  'package.json': FileJson,
  'package-lock.json': FileLock,
  'pnpm-lock.yaml': FileLock,
  'yarn.lock': FileLock,
  'tsconfig.json': FileCog,
  'readme.md': FileText,
  license: FileText,
};

const EXT_MAP: Record<string, LucideIcon> = {
  ts: FileCode,
  tsx: FileCode,
  js: FileCode,
  jsx: FileCode,
  mjs: FileCode,
  cjs: FileCode,
  rs: FileCode,
  go: FileCode,
  py: FileCode,
  rb: FileCode,
  java: FileCode,
  kt: FileCode,
  swift: FileCode,
  c: FileCode,
  cpp: FileCode,
  h: FileCode,
  hpp: FileCode,
  php: FileCode,
  lua: FileCode,
  zig: FileCode,
  vue: FileCode,
  svelte: FileCode,
  sh: FileTerminal,
  bash: FileTerminal,
  zsh: FileTerminal,
  fish: FileTerminal,
  json: FileJson,
  jsonc: FileJson,
  yaml: Braces,
  yml: Braces,
  toml: Braces,
  xml: Braces,
  html: FileCode,
  css: FileType,
  scss: FileType,
  sass: FileType,
  less: FileType,
  md: FileText,
  mdx: FileText,
  txt: FileText,
  rst: FileText,
  png: FileImage,
  jpg: FileImage,
  jpeg: FileImage,
  gif: FileImage,
  webp: FileImage,
  svg: FileImage,
  ico: FileImage,
  avif: FileImage,
  zip: FileArchive,
  tar: FileArchive,
  gz: FileArchive,
  rar: FileArchive,
  '7z': FileArchive,
  sql: Hash,
  graphql: Hash,
  prisma: Hash,
};

export function getFileIcon(filename: string): LucideIcon {
  const lower = filename.toLowerCase();

  if (NAME_MAP[lower]) {
    return NAME_MAP[lower];
  }
  if (lower.startsWith('dockerfile')) {
    return FileTerminal;
  }
  if (lower.startsWith('.env')) {
    return FileLock;
  }

  const ext = lower.split('.').pop() ?? '';
  return EXT_MAP[ext] ?? FileText;
}
