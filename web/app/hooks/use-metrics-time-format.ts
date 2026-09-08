import { useCallback } from 'react';
import { format, isValid } from 'date-fns';
import { parseUTC } from '~/utils/date';

export function useMetricsTimeFormat(rangeHours: number) {
  return useCallback(
    (value: unknown) => {
      if (typeof value !== 'string' && !(value instanceof Date)) {
        return '';
      }

      const date = parseUTC(value);
      if (!isValid(date)) {
        return '';
      }

      return rangeHours > 24
        ? format(date, 'MMM d, HH:mm')
        : format(date, 'HH:mm');
    },
    [rangeHours]
  );
}
