import { type ResourcesPayload } from '~/queries/services';

export const BYTES_PER_MB = 1024 * 1024;
export const NANO_PER_CORE = 1_000_000_000;

export const MIN_MEMORY_MB = 64;
export const MIN_CPU_CORES = 0.1;
export const MIN_SHM_MB = 64;
export const MIN_PIDS = 64;

export function parseMemoryMbToBytes(input: string): number | null {
  const trimmed = input.trim();
  if (!trimmed) {
    return null;
  }
  const n = Number(trimmed);
  if (!Number.isFinite(n) || n <= 0) {
    return null;
  }
  return Math.round(n * BYTES_PER_MB);
}

export function parseCoresToNano(input: string): number | null {
  const trimmed = input.trim();
  if (!trimmed) {
    return null;
  }
  const n = Number(trimmed);
  if (!Number.isFinite(n) || n <= 0) {
    return null;
  }
  return Math.round(n * NANO_PER_CORE);
}

export function parsePositiveInt(input: string): number | null {
  const trimmed = input.trim();
  if (!trimmed) {
    return null;
  }
  const n = Number(trimmed);
  if (!Number.isInteger(n) || n <= 0) {
    return null;
  }
  return n;
}

export function buildResourcesPayload(
  memoryMb: string,
  cpuCores: string,
  shmMb: string,
  pidsLimit: string
): { payload: ResourcesPayload | null; error: string | null } {
  const memory_bytes = parseMemoryMbToBytes(memoryMb);
  const nano_cpus = parseCoresToNano(cpuCores);
  const shm_size = parseMemoryMbToBytes(shmMb);
  const pids_limit = parsePositiveInt(pidsLimit);

  if (memoryMb.trim() && memory_bytes === null) {
    return { payload: null, error: 'Memory must be a positive number (MB).' };
  }
  if (cpuCores.trim() && nano_cpus === null) {
    return { payload: null, error: 'CPU must be a positive number of cores.' };
  }
  if (shmMb.trim() && shm_size === null) {
    return { payload: null, error: 'SHM must be a positive number (MB).' };
  }
  if (pidsLimit.trim() && pids_limit === null) {
    return { payload: null, error: 'PIDs limit must be a positive integer.' };
  }

  if (memory_bytes !== null && memory_bytes < MIN_MEMORY_MB * BYTES_PER_MB) {
    return {
      payload: null,
      error: `Memory must be at least ${MIN_MEMORY_MB} MB.`,
    };
  }
  if (nano_cpus !== null && nano_cpus < MIN_CPU_CORES * NANO_PER_CORE) {
    return {
      payload: null,
      error: `CPU must be at least ${MIN_CPU_CORES} cores.`,
    };
  }
  if (shm_size !== null && shm_size < MIN_SHM_MB * BYTES_PER_MB) {
    return { payload: null, error: `SHM must be at least ${MIN_SHM_MB} MB.` };
  }
  if (pids_limit !== null && pids_limit < MIN_PIDS) {
    return { payload: null, error: `PIDs limit must be at least ${MIN_PIDS}.` };
  }

  if (
    memory_bytes === null &&
    nano_cpus === null &&
    shm_size === null &&
    pids_limit === null
  ) {
    return { payload: null, error: null };
  }

  return {
    payload: { memory_bytes, nano_cpus, pids_limit, shm_size },
    error: null,
  };
}
