.PHONY: all build test lint license check-fmt check-clippy actor-bundle

BUILTIN_ACTORS_DIR:=../builtin-actors
BUILTIN_ACTORS_CODE:=$(shell find $(BUILTIN_ACTORS_DIR) -type f -name "*.rs" | grep -v target)
BUILTIN_ACTORS_BUNDLE:=$(shell pwd)/$(BUILTIN_ACTORS_DIR)/output/bundle.car

IPC_ACTORS_DIR:=$(shell pwd)/../ipc-solidity-actors
IPC_ACTORS_CODE:=$(shell find $(IPC_ACTORS_DIR) -type f -name "*.sol")
IPC_ACTORS_BUILD:=fendermint/vm/ipc_actors/build.rs
IPC_ACTORS_ABI:=$(IPC_ACTORS_DIR)/out/.compile.abi

FENDERMINT_CODE:=$(shell find . -type f \( -name "*.rs" -o -name "Cargo.toml" \) | grep -v target)

all: test build

build:
	cargo build --release

# Using --release for testing because wasm can otherwise be slow.
test: $(BUILTIN_ACTORS_BUNDLE)
	FM_BUILTIN_ACTORS_BUNDLE=$(BUILTIN_ACTORS_BUNDLE) cargo test --release --workspace --exclude smoke-test

e2e: docker-build
	cd fendermint/testing/smoke-test && cargo make

clean:
	cargo clean
	cd $(BUILTIN_ACTORS_DIR) && cargo clean
	rm $(BUILTIN_ACTORS_BUNDLE)

lint: \
	license \
	check-fmt \
	check-clippy

license:
	./scripts/add_license.sh

check-fmt:
	cargo fmt --all --check

check-clippy:
	cargo clippy --all --tests -- -D clippy::all

docker-build: $(BUILTIN_ACTORS_BUNDLE) $(FENDERMINT_CODE)
	mkdir -p docker/.artifacts

	cp $(BUILTIN_ACTORS_BUNDLE) docker/.artifacts

	if [ -z "$${GITHUB_ACTIONS}" ]; then \
		DOCKER_FILE=local ; \
	else \
		$(MAKE) --no-print-directory build && \
		cp ./target/release/fendermint docker/.artifacts && \
		DOCKER_FILE=ci ; \
	fi && \
	DOCKER_BUILDKIT=1 \
	docker build \
		-f docker/$${DOCKER_FILE}.Dockerfile \
		-t fendermint:latest $(PWD)

	rm -rf docker/.artifacts


# Build a bundle CAR; this is so we don't have to have a project reference,
# which means we are not tied to the release cycle of both FVM _and_ actors;
# so long as they work together.
actor-bundle: $(BUILTIN_ACTORS_BUNDLE)

$(BUILTIN_ACTORS_BUNDLE): $(BUILTIN_ACTORS_CODE)
	if [ ! -d $(BUILTIN_ACTORS_DIR) ]; then \
		mkdir -p $(BUILTIN_ACTORS_DIR) && \
		cd $(BUILTIN_ACTORS_DIR) && \
		cd .. && \
		git clone https://github.com/filecoin-project/builtin-actors.git; \
	fi
	cd $(BUILTIN_ACTORS_DIR) && \
	git checkout next && \
	git pull && \
	rustup target add wasm32-unknown-unknown && \
	cargo run --release -- -o output/$(shell basename $@)


# Compile the ABI artifacts for the IPC Solidity actors.
ipc-actors-abi: $(IPC_ACTORS_ABI)
	cargo build --release -p fendermint_vm_ipc_actors

# Clone the IPC Solidity actors if necessary and compile the ABI, putting down a marker at the end.
$(IPC_ACTORS_ABI): $(IPC_ACTORS_CODE) | forge
	if [ ! -d $(IPC_ACTORS_DIR) ]; then \
		mkdir -p $(IPC_ACTORS_DIR) && \
		cd $(IPC_ACTORS_DIR) && \
		cd .. && \
		git clone https://github.com/consensus-shipyard/ipc-solidity-actors.git; \
	fi
	cd $(IPC_ACTORS_DIR) && git pull
	make -C $(IPC_ACTORS_DIR) compile-abi
	touch $@


# Forge is used by the ipc-solidity-actors compilation steps.
.PHONY: forge
forge:
	@if [ -z "$(shell which forge)" ]; then \
		echo "Please install Foundry. See https://book.getfoundry.sh/getting-started/installation"; \
		exit 1; \
	fi
