FROM node:22-alpine
WORKDIR /app
COPY web/package*.json ./
RUN npm install
COPY web ./
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]
