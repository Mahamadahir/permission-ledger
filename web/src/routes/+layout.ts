// The dashboard is private and every request goes to a cross-origin API using
// cookies that server rendering cannot forward, so run as a client-only SPA.
export const ssr = false;
