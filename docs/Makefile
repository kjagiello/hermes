IMAGE_NAME := hermes-docs:latest

# Extract the app version from Cargo.toml as default
HERMES_VERSION ?= $(shell \
	cargo metadata \
		--format-version 1 \
		--no-deps \
		--manifest-path ../Cargo.toml \
	| jq -r '.packages[0].version' \
)

ifeq ($(HERMES_VERSION),)
    $(error Could not determine the version of Hermes)
endif

DOCKER_RUN_PORT := 8000
DOCKER_RUN_MOUNTS := -v $(CURDIR):/app/docs \
										 -v $(CURDIR)/../plugin.yaml:/app/plugin.yaml \
										 -v $(CURDIR)/../mkdocs.yaml:/app/mkdocs.yaml
DOCKER_RUN_OPTS := --rm \
									 $(DOCKER_RUN_MOUNTS) \
									 -p $(DOCKER_RUN_PORT):8000 \
									 -e HERMES_VERSION=$(HERMES_VERSION)

.PHONY: build
build:
	docker buildx install
	docker build \
		--load \
		--cache-from "type=local,src=/tmp/.buildx-cache" \
		--cache-to "type=local,dest=/tmp/.buildx-cache" \
		--build-arg HERMES_VERSION=$(HERMES_VERSION) \
		-f ./Dockerfile \
		-t $(IMAGE_NAME) \
		..

.PHONY: serve
serve: build
	docker run $(DOCKER_RUN_OPTS) $(IMAGE_NAME) mkdocs serve --dev-addr 0.0.0.0:8000

.PHONY: deploy
deploy: DOCKER_RUN_OPTS += -v $(CURDIR)/../.git:/app/.git
deploy: build
	docker run $(DOCKER_RUN_OPTS) $(IMAGE_NAME) \
		mike deploy -F mkdocs.yaml \
		$(OPTS) $(VERSION) $(ALIAS)
	@if [ -n "$(SET_DEFAULT)" ]; then \
		docker run $(DOCKER_RUN_OPTS) $(IMAGE_NAME) \
			mike set-default -F mkdocs.yaml $(VERSION); \
	fi

.PHONY: shell
shell: build
	docker run -it $(DOCKER_RUN_OPTS) $(IMAGE_NAME) bash
