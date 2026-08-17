CARGO := cargo
VORPAL := vorpal
VORPAL_ARTIFACT := dev
VORPAL_FLAGS :=

.DEFAULT_GOAL := check

.PHONY: check fmt fmt-check clippy build vorpal-build vorpal-prepare

check:
	$(CARGO) check

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --all-targets --all-features

build:
	$(CARGO) build

vorpal-build:
	$(VORPAL) build $(VORPAL_FLAGS) $(VORPAL_ARTIFACT)

vorpal-prepare:
	$(VORPAL) prepare $(VORPAL_FLAGS) $(VORPAL_ARTIFACT)
