# Stage 1: Build the application
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY package.json ./

# Install dependencies
RUN npm install

# Copy source code
COPY . .

# Build the application
RUN npm run build

# Stage 2: Production image
FROM node:20-alpine AS runner

WORKDIR /app

# Create non-root user
RUN addgroup --system --gid 1001 nodejs && \
    adduser --system --uid 1001 sveltekit

# Copy built application
COPY --from=builder --chown=sveltekit:nodejs /app/build ./build
COPY --from=builder --chown=sveltekit:nodejs /app/package.json ./

# Install only production dependencies
RUN npm install --omit=dev

USER sveltekit

# Expose the port
EXPOSE 3000

# Set environment variables
ENV NODE_ENV=production
ENV PORT=3000
ENV HOST=0.0.0.0

# Start the server
CMD ["node", "build"]
