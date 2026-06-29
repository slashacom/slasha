import { useState } from 'react';

export function usePagination<T>(items: T[], initialLimit = 10) {
  const [page, setPage] = useState(0);
  const [limit, setLimitState] = useState(initialLimit);
  const pageCount = Math.max(1, Math.ceil(items.length / limit));
  const currentPage = Math.min(page, pageCount - 1);

  return {
    rows: items.slice(currentPage * limit, currentPage * limit + limit),
    page: currentPage,
    pageCount,
    limit,
    setLimit(size: number) {
      setLimitState(size);
      setPage(0);
    },
    previousPage() {
      setPage((value) => Math.max(0, value - 1));
    },
    nextPage() {
      setPage((value) => Math.min(value + 1, pageCount - 1));
    },
  };
}
