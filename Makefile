# modgen: validate a Cairn module set and emit what each surface needs.
#
#   make check      everything CI runs
#   make validate   every manifest, against the pinned contracts
#   make list       what is here
#   make identity   the module-set identity string
#   make vectors    contracts/module/v1 own vectors, against this implementation
#   make fields     the capture-field union, for the firmware enginegen
#   make go         the module set as Go data, for the server

MODULES_DIR := modules
MODGEN     := tools/modgen/target/release/modgen
BUILD      := build
SELECT     ?= all

.PHONY: help check modgen contracts validate list identity vectors fields go test clean

help:
	@sed -n '2,9p' Makefile

# Always run cargo: it is a no-op when nothing changed, and it makes a stale binary
# impossible.
modgen:
	cargo build --release --locked --quiet --manifest-path tools/modgen/Cargo.toml

# A no-op when .contracts is already at the pinned commit, so this is cheap to depend on.
contracts:
	scripts/fetch-contracts.sh

validate: modgen contracts
	$(MODGEN) validate --dir $(MODULES_DIR)

list: modgen
	$(MODGEN) list --dir $(MODULES_DIR)

identity: modgen
	$(MODGEN) identity --dir $(MODULES_DIR)

vectors: modgen contracts
	$(MODGEN) vectors

fields: modgen contracts
	$(MODGEN) gen --target fields --modules '$(SELECT)' --out $(BUILD)/fields.txt

go: modgen contracts
	$(MODGEN) gen --target go --modules '$(SELECT)' --out $(BUILD)/modules_gen.go

test: contracts
	cargo test --locked --quiet --manifest-path tools/modgen/Cargo.toml

# What CI runs. The emitters are exercised too: a generator that is never run is a
# generator that does not work.
check: validate vectors test fields go
	@echo "modgen: $$($(MODGEN) identity --dir $(MODULES_DIR))"

clean:
	rm -rf $(BUILD)
