FROM python:3.9.9-slim-buster

ARG HERMES_VERSION
ENV HERMES_VERSION $HERMES_VERSION

ENV PIP_PIP_VERSION 21.3.1
ENV APT_GIT_VERSION 1:2.20.*

RUN --mount=type=cache,target=/var/cache/apt --mount=type=cache,target=/var/lib/apt \
    set -x && apt-get update && apt-get install --no-install-recommends -y \
    git=$APT_GIT_VERSION

WORKDIR /app
ADD docs/requirements.txt .
RUN --mount=type=cache,target=/root/.cache/pip \
    set -x && \
    pip install pip==$PIP_PIP_VERSION && \
    pip install -r requirements.txt
