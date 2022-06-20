.PHONY: fmt
fmt:
	find -type f -name "*.rs" -not -path "*target*" -exec rustfmt --edition 2021 {} \;

.PHONY: lint
lint:
	cargo +nightly clippy --fix -Z unstable-options --release --all

.PHONY: build-docker
build-docker:
	DOCKER_BUILDKIT=1 docker \
		build \
		--compress \
		--memory 8g \
		--cpu-shares 4096 \
		--shm-size 8g \
		-t template-cli:latest \
		--ssh \
		--squash .
	docker image save template-cli:latest -o template_cli.tar
	pigz -f -9 template_cli.tar


.PHONY: build-cli
build-cli:
	./scripts/build_cli.sh

.PHONY: build-cli-debug
build-cli-debug:
	(cargo build ; cp target/debug/cli template-cli)
