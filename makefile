.DEFAULT_GOAL := check

# Artifact built by the vorpal-* targets; override for one-off builds, e.g.
# `make vorpal-build VORPAL_ARTIFACT=kubectl`.
VORPAL_ARTIFACT ?= dev

.PHONY: build check clippy fmt fmt-check registry-check vorpal-build vorpal-prepare

build:
	cargo build --locked

check:
	cargo check --locked

clippy:
	cargo clippy --locked -- --deny warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

registry-check:
	bash script/check-artifact-registry.sh

vorpal-build:
	vorpal build $(VORPAL_ARTIFACT)

vorpal-prepare:
	vorpal prepare $(VORPAL_ARTIFACT)
