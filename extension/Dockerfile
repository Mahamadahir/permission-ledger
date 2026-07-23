FROM node:22-alpine
WORKDIR /app
COPY extension/package*.json ./
RUN npm install
COPY extension ./
CMD ["npm", "run", "build"]
