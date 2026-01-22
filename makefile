ARCH := $(shell uname -m | tr '[:upper:]' '[:lower:]' | sed 's/arm64/aarch64/')
LIMA_ARCH := $(ARCH)
LIMA_CPUS := 8
LIMA_DISK := 100
LIMA_MEMORY := 8
WORK_DIR := $(shell pwd)
VORPAL_ARTIFACT := dev

# Lima environment

lima-clean:
	limactl stop "artifacts-$(LIMA_ARCH)" || true
	limactl delete "artifacts-$(LIMA_ARCH)" || true

lima: lima-clean
	cat lima.yaml | limactl create --arch "$(LIMA_ARCH)" --cpus "$(LIMA_CPUS)" --disk "$(LIMA_DISK)" --memory "$(LIMA_MEMORY)" --name "artifacts-$(LIMA_ARCH)" -
	limactl start "artifacts-$(LIMA_ARCH)"
	limactl shell "artifacts-$(LIMA_ARCH)" $(WORK_DIR)/script/lima.sh deps
	limactl stop "artifacts-$(LIMA_ARCH)"
	limactl start "artifacts-$(LIMA_ARCH)"

lima-sync:
	limactl shell "artifacts-$(LIMA_ARCH)" ./script/lima.sh sync

lima-vorpal:
	limactl shell "artifacts-$(LIMA_ARCH)" bash -c 'cd ~/vorpal && target/debug/vorpal build $(VORPAL_FLAGS) $(VORPAL_ARTIFACT)'

lima-vorpal-start:
	limactl shell "artifacts-$(LIMA_ARCH)" bash -c '~/vorpal/target/debug/vorpal services start $(VORPAL_FLAGS)'
