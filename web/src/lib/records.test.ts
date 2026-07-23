import { describe, expect, it } from 'vitest';
import { buildRecordQuery, formatDate, serviceInitial } from './records';

describe('buildRecordQuery', () => {
  it('is empty when no filters are set', () => {
    expect(buildRecordQuery({})).toBe('');
  });

  it('includes only the filters that have values', () => {
    const query = buildRecordQuery({
      search: 'acme',
      riskFilter: 'high',
      reviewDueOnly: true,
      categoryFilter: '',
      statusFilter: ''
    });
    const params = new URLSearchParams(query);
    expect(params.get('q')).toBe('acme');
    expect(params.get('risk_level')).toBe('high');
    expect(params.get('review_due')).toBe('true');
    expect(params.has('category_id')).toBe(false);
    expect(params.has('status')).toBe(false);
  });

  it('omits review_due when the flag is false', () => {
    expect(buildRecordQuery({ reviewDueOnly: false })).toBe('');
  });
});

describe('formatDate', () => {
  it('returns "Not set" for null', () => {
    expect(formatDate(null)).toBe('Not set');
  });

  it('formats an ISO date in en-GB', () => {
    expect(formatDate('2026-03-05')).toBe('05 Mar 2026');
  });
});

describe('serviceInitial', () => {
  it('uppercases the first non-space character', () => {
    expect(serviceInitial('  acme corp')).toBe('A');
  });

  it('falls back to P when empty', () => {
    expect(serviceInitial('   ')).toBe('P');
  });
});
