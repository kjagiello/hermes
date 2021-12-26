VERSION := latest
IMAGE_NAMESPACE := ghcr.io/kjagiello
IMAGE_NAME := hermes

KUBECTX := $(shell [[ "`which kubectl`" != '' ]] && kubectl config current-context || echo none)
K3D := $(shell [[ "$(KUBECTX)" == "k3d-"* ]] && echo true || echo false)
K3D_CLUSTER_NAME ?= k3s-default
SED := $(shell [[ "`which gsed`" != '' ]] && echo "gsed" || echo "sed")

require_var = $(if $(value $(1)),,$(error $(1) not set))

.PHONY: build-image
build-image: PLATFORM=linux/amd64
build-image:
	docker buildx install
	docker build \
		--load \
		--platform $(PLATFORM) \
		--cache-from "type=local,src=/tmp/.buildx-cache" \
		--cache-to "type=local,dest=/tmp/.buildx-cache" \
		-t $(IMAGE_NAMESPACE)/$(IMAGE_NAME):$(VERSION) \
		.

.PHONY: install-image
install-image: build-image
	if [ $(K3D) = true ]; then k3d image import -c $(K3D_CLUSTER_NAME) $(IMAGE_NAMESPACE)/$(IMAGE_NAME):$(VERSION); fi

.PHONY: install
install: SLACK_TOKEN=
install: install-image
	$(call require_var,SLACK_TOKEN)
	envsubst < manifests/install.yaml | kubectl apply -f -

.PHONY: update-version
update-version: VERSION=
update-version:
	$(call require_var,VERSION)
	@$(SED) -i -E "s/^version = \".+\"/version = \"$$VERSION\"/g" Cargo.toml
	@$(SED) -i -E "0,/version = \".+\"/s//version = \"$$VERSION\"/" Cargo.lock
	@$(SED) -i -E "s/ghcr\.io\/.+:[^\s]+/ghcr.io\/kjagiello\/hermes:$$VERSION/" plugin.yaml
	@echo "The version has updated to $$VERSION"
