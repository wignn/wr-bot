#!/bin/bash

set -e

# Configuration
APP_NAME="worm-bot"
IMAGE_NAME="worm-bot"
CONTAINER_NAME="worm-bot-container"
VERSION=$(git rev-parse --short HEAD 2>/dev/null || echo "latest")

echo "🚀 Starting deployment for $APP_NAME..."

# Stop and remove existing container
echo "📦 Stopping existing container..."
sudo docker stop $CONTAINER_NAME 2>/dev/null || true
sudo docker rm $CONTAINER_NAME 2>/dev/null || true

# Build image
echo "🔨 Building Docker image..."
sudo docker build -t $IMAGE_NAME:$VERSION -t $IMAGE_NAME:latest .

# Run container
echo "▶️  Starting container..."
sudo docker run -d \
  --name $CONTAINER_NAME \
  --restart unless-stopped \
  --env-file .env \
  $IMAGE_NAME:latest

# Check status
echo "✅ Checking container status..."
sleep 2
sudo docker ps | grep $CONTAINER_NAME

echo "🎉 Deployment completed!"
echo "📊 View logs: docker logs -f $CONTAINER_NAME"