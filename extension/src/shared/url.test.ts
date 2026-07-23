import { describe, expect, it } from 'vitest';
import { hostFromUrl, nextMonth } from './url';

describe('hostFromUrl', () => {
  it('strips a leading www.', () => {
    expect(hostFromUrl('https://www.example.com/path')).toBe('example.com');
  });

  it('keeps a bare host', () => {
    expect(hostFromUrl('https://app.example.com')).toBe('app.example.com');
  });

  it('returns empty string for an invalid URL', () => {
    expect(hostFromUrl('not a url')).toBe('');
  });
});

describe('nextMonth', () => {
  it('advances one month and formats as an ISO date', () => {
    expect(nextMonth(new Date('2026-01-15T00:00:00Z'))).toBe('2026-02-15');
  });

  it('rolls over the year in December', () => {
    expect(nextMonth(new Date('2026-12-10T00:00:00Z'))).toBe('2027-01-10');
  });
});
