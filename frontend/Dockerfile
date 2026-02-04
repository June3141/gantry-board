FROM node:22-slim

WORKDIR /app

# Copy package files first for dependency caching
COPY package.json package-lock.json ./

# Install dependencies
RUN npm ci

# Copy source code
COPY . .

EXPOSE 5173

# Vite dev server with host binding for Docker
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]
