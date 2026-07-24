# Production image: build the SPA, then serve it from nginx with /api proxied
# to the backend so the app and API share one origin.
FROM node:22-alpine AS build
WORKDIR /app
COPY web/package*.json ./
RUN npm ci
COPY web ./
RUN npm run build

FROM nginx:alpine
# The entrypoint runs envsubst over templates/ at start; the filter keeps it to
# BACKEND_UPSTREAM so nginx's own $host/$uri/$scheme are left untouched.
COPY web/nginx.conf.template /etc/nginx/templates/default.conf.template
ENV NGINX_ENVSUBST_FILTER=BACKEND_UPSTREAM
COPY --from=build /app/build /usr/share/nginx/html
EXPOSE 80
