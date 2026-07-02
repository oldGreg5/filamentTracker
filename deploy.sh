#!/bin/bash
set -e  # exit immediately if any command fails

# 1. Build locally
docker buildx build --platform linux/amd64 -t oldgreg5/filament-tracker:latest --push .

export SSHPASS='Thai0lai503217kicak'

# 2 & 3. SSH into QNAP and stop the container remotely
sshpass -e ssh -p 924 oldgregx@192.168.100.104 "
  /share/CACHEDEV1_DATA/.qpkg/container-station/bin/docker stop filament-tracker && \
  /share/CACHEDEV1_DATA/.qpkg/container-station/bin/docker rm filament-tracker && \
  /share/CACHEDEV1_DATA/.qpkg/container-station/bin/docker rmi oldgreg5/filament-tracker && \
  /share/CACHEDEV1_DATA/.qpkg/container-station/bin/docker pull oldgreg5/filament-tracker:latest && \
  /share/CACHEDEV1_DATA/.qpkg/container-station/bin/docker run -d \
  --name filament-tracker \
  -p 8090:8080 \
  -v /share/Container/shared_data/filament-tracker:/data \
  --restart always \
  oldgreg5/filament-tracker:latest
"