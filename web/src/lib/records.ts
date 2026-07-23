export type RecordFilters = {
  search?: string;
  categoryFilter?: string;
  statusFilter?: string;
  riskFilter?: string;
  sourceFilter?: string;
  reviewDueOnly?: boolean;
};

export function buildRecordQuery(filters: RecordFilters): string {
  const params = new URLSearchParams();
  if (filters.search) params.set('q', filters.search);
  if (filters.categoryFilter) params.set('category_id', filters.categoryFilter);
  if (filters.statusFilter) params.set('status', filters.statusFilter);
  if (filters.riskFilter) params.set('risk_level', filters.riskFilter);
  if (filters.sourceFilter) params.set('source', filters.sourceFilter);
  if (filters.reviewDueOnly) params.set('review_due', 'true');
  return params.toString();
}

export function formatDate(value: string | null): string {
  if (!value) return 'Not set';
  return new Intl.DateTimeFormat('en-GB', { day: '2-digit', month: 'short', year: 'numeric' }).format(
    new Date(value)
  );
}

export function serviceInitial(value: string): string {
  return value.trim().slice(0, 1).toUpperCase() || 'P';
}
