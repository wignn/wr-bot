IMAGE_NAME = wign/bot-discord
IMAGE_TAG = latest

run:
	set AUDIOPUS_SYS_USE_PKG_CONFIG=1 && cargo run

dev:
	docker compose up --build

down:
	docker compose down

logs:
	docker compose logs -f

build:
	docker build -t $(IMAGE_NAME):$(IMAGE_TAG) .

build-multi:
	docker buildx build --platform linux/amd64,linux/arm64 -t $(IMAGE_NAME):$(IMAGE_TAG) --push .

build-amd64:
	docker buildx build --platform linux/amd64 -t $(IMAGE_NAME):$(IMAGE_TAG) --push .

build-arm64:
	docker buildx build --platform linux/arm64 -t $(IMAGE_NAME):$(IMAGE_TAG) --push .

push:
	docker push $(IMAGE_NAME):$(IMAGE_TAG)

setup-buildx:
	docker buildx create --name multibuilder --driver docker-container --use || true
	docker buildx inspect --bootstrap

clean:
	docker compose down -v --rmi local
	docker system prune -f

help:
	@echo "Available commands:"
	@echo "  make run          - Run locally with cargo"
	@echo "  make dev          - Run with docker compose (build + up)"
	@echo "  make down         - Stop docker compose"
	@echo "  make logs         - View docker compose logs"
	@echo ""
	@echo "  make build        - Build docker image for current platform"
	@echo "  make build-multi  - Build and push for amd64 + arm64"
	@echo "  make build-amd64  - Build and push for amd64 only"
	@echo "  make build-arm64  - Build and push for arm64 only"
	@echo ""
	@echo "  make push         - Push image to Docker Hub"
	@echo "  make setup-buildx - Setup buildx for multi-platform builds"
	@echo "  make clean        - Clean up docker resources"

.PHONY: run dev down logs build build-multi build-amd64 build-arm64 push setup-buildx clean help
