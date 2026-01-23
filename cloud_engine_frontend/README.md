# Cloud Engine Controller Frontend

A modern Svelte-based frontend for the Cloud Engine Controller, enabling users to manage GCP VMs and their ICP node associations through a web interface.

## Features

- **Internet Identity Authentication**: Secure login via IC's Internet Identity
- **VM Management**: List, provision, and delete GCP virtual machines
- **Node Visualization**: View ICP nodes from the NNS registry
- **Subnet Management**: Create and delete subnet proposals via NNS
- **User Profile**: Configure GCP account and node operator settings
- **Modern UI**: Built with SvelteKit, TypeScript, and Tailwind CSS

## Development

### Prerequisites

- Node.js 20+
- npm or yarn

### Setup

```bash
# Install dependencies
npm install

# Start development server
npm run dev
```

The development server runs at `http://localhost:3000` and proxies API requests to `http://localhost:8000` (the backend).

### Environment Variables

Create a `.env` file based on `.env.example`:

```bash
# Backend API URL (defaults to /api which uses the vite proxy in development)
VITE_API_URL=http://localhost:8000

# Internet Identity URL (defaults to mainnet)
VITE_II_URL=https://identity.ic0.app
```

### Build

```bash
# Build for production
npm run build

# Preview production build
npm run preview
```

## Docker

### Build Image

```bash
docker build -t cloud-engine-frontend:latest .
```

### Run Container

```bash
docker run -p 3000:3000 \
  -e VITE_API_URL=http://backend:8000 \
  cloud-engine-frontend:latest
```

## Kubernetes Deployment

### Prerequisites

- Kubernetes cluster
- kubectl configured
- Backend service deployed (see backend README)

### Deployment Manifests

Create the following Kubernetes manifests:

**namespace.yaml**
```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: cloud-engine
```

**frontend-deployment.yaml**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cloud-engine-frontend
  namespace: cloud-engine
  labels:
    app: cloud-engine-frontend
spec:
  replicas: 2
  selector:
    matchLabels:
      app: cloud-engine-frontend
  template:
    metadata:
      labels:
        app: cloud-engine-frontend
    spec:
      containers:
        - name: frontend
          image: ghcr.io/dfinity/dre/cloud-engine-frontend:latest
          ports:
            - containerPort: 3000
              name: http
          env:
            - name: NODE_ENV
              value: "production"
            - name: PORT
              value: "3000"
          resources:
            requests:
              memory: "128Mi"
              cpu: "100m"
            limits:
              memory: "256Mi"
              cpu: "500m"
          livenessProbe:
            httpGet:
              path: /
              port: 3000
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /
              port: 3000
            initialDelaySeconds: 5
            periodSeconds: 10
```

**frontend-service.yaml**
```yaml
apiVersion: v1
kind: Service
metadata:
  name: cloud-engine-frontend
  namespace: cloud-engine
spec:
  selector:
    app: cloud-engine-frontend
  ports:
    - port: 80
      targetPort: 3000
      name: http
  type: ClusterIP
```

**ingress.yaml** (combined frontend and backend)
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: cloud-engine
  namespace: cloud-engine
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - cloud-engine.example.com
      secretName: cloud-engine-tls
  rules:
    - host: cloud-engine.example.com
      http:
        paths:
          # Frontend routes
          - path: /
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-frontend
                port:
                  number: 80
          # Backend API routes
          - path: /api
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-backend
                port:
                  number: 80
```

### Deploy

```bash
# Create namespace
kubectl apply -f namespace.yaml

# Deploy frontend
kubectl apply -f frontend-deployment.yaml
kubectl apply -f frontend-service.yaml

# Deploy ingress (after backend is deployed)
kubectl apply -f ingress.yaml

# Check status
kubectl get pods -n cloud-engine
kubectl get svc -n cloud-engine
```

### Using Helm (Alternative)

If you prefer Helm, create a values file:

```yaml
# values.yaml
frontend:
  replicaCount: 2
  image:
    repository: ghcr.io/dfinity/dre/cloud-engine-frontend
    tag: latest
  service:
    type: ClusterIP
    port: 80
  resources:
    limits:
      cpu: 500m
      memory: 256Mi
    requests:
      cpu: 100m
      memory: 128Mi

ingress:
  enabled: true
  className: nginx
  hosts:
    - host: cloud-engine.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: cloud-engine-tls
      hosts:
        - cloud-engine.example.com
```

## Project Structure

```
cloud_engine_frontend/
├── src/
│   ├── app.html              # HTML template
│   ├── app.css               # Global styles (Tailwind)
│   ├── lib/
│   │   ├── api.ts            # API client
│   │   ├── auth.ts           # Internet Identity integration
│   │   ├── stores.ts         # Svelte stores
│   │   ├── types.ts          # TypeScript types
│   │   └── components/       # Reusable components
│   └── routes/
│       ├── +layout.svelte    # Root layout
│       ├── +page.svelte      # Dashboard/login
│       ├── profile/          # Profile settings
│       ├── vms/              # VM management
│       ├── nodes/            # Node listing
│       └── subnets/          # Subnet management
├── static/                   # Static assets
├── Dockerfile
├── package.json
├── svelte.config.js
├── tailwind.config.js
└── vite.config.ts
```

## License

Apache-2.0
