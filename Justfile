build:
    podman build . -t docker.io/follpvosten/godfishbot-ng:latest

push:
    podman push docker.io/follpvosten/godfishbot-ng:latest
