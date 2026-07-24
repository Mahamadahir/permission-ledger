export type User = { id: string; email: string; display_name: string | null };

export type Category = { id: string; slug: string; name: string };

export type RecordItem = {
  id: string;
  service_name: string;
  website_url: string;
  category_id: string;
  category_name: string;
  consent_type: string;
  date_given: string;
  review_date: string | null;
  expiry_date: string | null;
  status: string;
  risk_level: string;
  source: string;
  notes: string | null;
};

export type Summary = {
  active: number;
  review_due: number;
  expired: number;
  revoked: number;
  high_risk: number;
};

export type Dashboard = {
  summary: Summary;
  recent: RecordItem[];
  categories: { name: string; count: number }[];
  services: { name: string; count: number }[];
};

export type Device = {
  id: string;
  name: string;
  last_used_at: string | null;
  revoked_at: string | null;
  created_at: string;
};

export type Settings = {
  display_name: string | null;
  timezone: string;
  review_reminder_days: number;
};

export type RecordForm = {
  id: string;
  service_name: string;
  website_url: string;
  consent_type: string;
  category_id: string;
  date_given: string;
  review_date: string;
  expiry_date: string;
  status: string;
  risk_level: string;
  notes: string;
};

export function emptyRecordForm(): RecordForm {
  return {
    id: '',
    service_name: '',
    website_url: '',
    consent_type: '',
    category_id: '',
    date_given: new Date().toISOString().slice(0, 10),
    review_date: '',
    expiry_date: '',
    status: 'active',
    risk_level: 'medium',
    notes: ''
  };
}
