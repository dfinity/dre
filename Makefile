compose:
	podman-compose -f docker-compose.yml -f docker-compose.dev.yml up --build

dev:
	yarn dev
