# Stage 1 - Create yarn install skeleton layer
FROM node:18-alpine AS packages

WORKDIR /app
COPY package.json yarn.lock ./

COPY packages packages
COPY plugins plugins

RUN find packages \! -name "package.json" -mindepth 2 -maxdepth 2 -exec rm -rf {} \+

# Stage 2 - Install dependencies and build packages
FROM node:18-alpine AS build

WORKDIR /app
COPY --from=packages /app .

# Install build dependencies more efficiently
RUN apk add --no-cache python3 make g++ sqlite-dev && \
    yarn config set python /usr/bin/python3 && \
    # Use yarn cache and parallel builds for faster installation
    yarn install --frozen-lockfile --network-timeout 600000 --prefer-offline && \
    # Clean up package cache to reduce layer size
    yarn cache clean

COPY . .

# Build with optimizations
RUN yarn tsc && \
    yarn --cwd packages/backend backstage-cli backend:bundle --build-dependencies

# Stage 3 - Build the actual backend image and install production dependencies
FROM node:18-alpine

WORKDIR /app

# Copy the install dependencies from the build stage and context
COPY --from=build /app/yarn.lock /app/package.json /app/packages/backend/dist/skeleton.tar.gz ./
RUN tar xzf skeleton.tar.gz && rm skeleton.tar.gz

# Install only production dependencies
RUN apk add --no-cache sqlite-dev && \
    yarn install --frozen-lockfile --production --network-timeout 600000 --prefer-offline && \
    yarn cache clean && \
    # Remove unnecessary files to reduce image size
    rm -rf /tmp/* /var/cache/apk/*

# Copy the built packages from the build stage
COPY --from=build /app/packages/backend/dist/bundle.tar.gz .
RUN tar xzf bundle.tar.gz && rm bundle.tar.gz

# Copy any other files that we need at runtime
COPY app-config.yaml ./
COPY app-config.production.yaml ./

# Use non-root user for security and performance
RUN addgroup -g 1001 -S nodejs && \
    adduser -S backstage -u 1001 && \
    chown -R backstage:nodejs /app

USER backstage

CMD ["node", "packages/backend", "--config", "app-config.yaml", "--config", "app-config.production.yaml"]
