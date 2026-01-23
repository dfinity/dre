# Cloud Engine Controller Backend

A backend service for managing Google Cloud Platform VMs and their association with Internet Computer nodes.

## Features

- **Internet Identity Authentication**: Validate IC delegations from frontend applications
- **GCP VM Management**: List, provision, and delete VMs in Google Cloud
- **ICP Node Mapping**: Map GCP VMs to ICP nodes using local registry sync
- **Subnet Management**: Create and delete subnets via NNS proposals

## API Documentation

The service exposes a Swagger UI at `/swagger-ui/` for interactive API documentation and testing.

## Running Locally

```bash
cargo run --release -- \
    --targets-dir /path/to/registry/store \
    --gcp-credentials-file /path/to/service-account.json
```

## Configuration

| Flag | Description | Default |
|------|-------------|---------|
| `--targets-dir` | Directory for registry local store | Required |
| `--port` | Server port | 8000 |
| `--gcp-credentials-file` | Path to GCP service account JSON | Optional (uses ADC) |
| `--poll-interval` | Registry sync interval | 30s |
| `--nns-url` | NNS URL for registry sync | https://ic0.app |
| `--users-state-file` | Path to users state file | Optional |

## Architecture

```
Frontend (with II) --> Backend API --> GCP Compute API
                                   --> ICP Registry (NNS)
                                   --> NNS Governance (Proposals)
```

## Docker

### Build Image

```bash
# From repository root
docker build -t cloud-engine-backend:latest -f rs/cloud-engine-controller-backend/Dockerfile .
```

### Run Container

```bash
docker run -p 8000:8000 \
  -v /path/to/gcp-credentials.json:/app/credentials.json:ro \
  -v /path/to/data:/app/data \
  -e GOOGLE_APPLICATION_CREDENTIALS=/app/credentials.json \
  cloud-engine-backend:latest \
  --targets-dir /app/registry \
  --gcp-credentials-file /app/credentials.json \
  --users-state-file /app/data/users.json
```

## Kubernetes Deployment

### Prerequisites

- Kubernetes cluster
- kubectl configured
- GCP service account credentials stored as a Kubernetes secret

### Create Secrets

First, create a secret for GCP credentials:

```bash
kubectl create namespace cloud-engine

kubectl create secret generic gcp-credentials \
  --namespace cloud-engine \
  --from-file=credentials.json=/path/to/your/service-account.json
```

### Deployment Manifests

**backend-deployment.yaml**
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cloud-engine-backend
  namespace: cloud-engine
  labels:
    app: cloud-engine-backend
spec:
  replicas: 2
  selector:
    matchLabels:
      app: cloud-engine-backend
  template:
    metadata:
      labels:
        app: cloud-engine-backend
    spec:
      containers:
        - name: backend
          image: ghcr.io/dfinity/dre/cloud-engine-backend:latest
          args:
            - --targets-dir
            - /app/registry
            - --port
            - "8000"
            - --gcp-credentials-file
            - /app/secrets/credentials.json
            - --users-state-file
            - /app/data/users.json
            - --poll-interval
            - 60s
          ports:
            - containerPort: 8000
              name: http
          env:
            - name: GOOGLE_APPLICATION_CREDENTIALS
              value: /app/secrets/credentials.json
            - name: RUST_LOG
              value: info
          volumeMounts:
            - name: gcp-credentials
              mountPath: /app/secrets
              readOnly: true
            - name: data
              mountPath: /app/data
            - name: registry
              mountPath: /app/registry
          resources:
            requests:
              memory: "256Mi"
              cpu: "100m"
            limits:
              memory: "512Mi"
              cpu: "1000m"
          livenessProbe:
            httpGet:
              path: /health
              port: 8000
            initialDelaySeconds: 30
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /health
              port: 8000
            initialDelaySeconds: 10
            periodSeconds: 10
      volumes:
        - name: gcp-credentials
          secret:
            secretName: gcp-credentials
        - name: data
          persistentVolumeClaim:
            claimName: cloud-engine-backend-data
        - name: registry
          emptyDir: {}
```

**backend-pvc.yaml**
```yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: cloud-engine-backend-data
  namespace: cloud-engine
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 1Gi
```

**backend-service.yaml**
```yaml
apiVersion: v1
kind: Service
metadata:
  name: cloud-engine-backend
  namespace: cloud-engine
spec:
  selector:
    app: cloud-engine-backend
  ports:
    - port: 80
      targetPort: 8000
      name: http
  type: ClusterIP
```

### Full Stack Deployment

Deploy both frontend and backend together:

```bash
# Create namespace
kubectl create namespace cloud-engine

# Create GCP credentials secret
kubectl create secret generic gcp-credentials \
  --namespace cloud-engine \
  --from-file=credentials.json=/path/to/service-account.json

# Apply backend resources
kubectl apply -f backend-pvc.yaml
kubectl apply -f backend-deployment.yaml
kubectl apply -f backend-service.yaml

# Apply frontend resources (from frontend README)
kubectl apply -f frontend-deployment.yaml
kubectl apply -f frontend-service.yaml

# Apply ingress
kubectl apply -f ingress.yaml

# Verify deployment
kubectl get pods -n cloud-engine
kubectl get svc -n cloud-engine
kubectl get ingress -n cloud-engine
```

### Combined Ingress Configuration

**ingress.yaml**
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: cloud-engine
  namespace: cloud-engine
  annotations:
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
    nginx.ingress.kubernetes.io/proxy-body-size: "10m"
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
          # Backend API routes - must be before frontend catch-all
          - path: /auth
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-backend
                port:
                  number: 80
          - path: /user
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-backend
                port:
                  number: 80
          - path: /vms
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-backend
                port:
                  number: 80
          - path: /nodes
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-backend
                port:
                  number: 80
          - path: /subnets
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-backend
                port:
                  number: 80
          - path: /swagger-ui
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-backend
                port:
                  number: 80
          - path: /api-docs
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-backend
                port:
                  number: 80
          - path: /metrics
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-backend
                port:
                  number: 80
          - path: /health
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-backend
                port:
                  number: 80
          # Frontend catch-all
          - path: /
            pathType: Prefix
            backend:
              service:
                name: cloud-engine-frontend
                port:
                  number: 80
```

### Monitoring

The backend exposes Prometheus metrics at `/metrics`. To scrape these with Prometheus:

```yaml
# ServiceMonitor for Prometheus Operator
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: cloud-engine-backend
  namespace: cloud-engine
spec:
  selector:
    matchLabels:
      app: cloud-engine-backend
  endpoints:
    - port: http
      path: /metrics
      interval: 30s
```

## Development

```bash
# Run tests
cargo test

# Run with hot reload
cargo watch -x run

# Run with specific arguments
cargo watch -x 'run -- --targets-dir ./data/registry --port 8000'
```

## License

Apache-2.0
