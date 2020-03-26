# This basic Makefile should be compatible with both BSD and GNU make.
BINARY=		s3du
CARGO=		cargo
MANDOC=		mandoc
MAN_DIR=	man
MAN_SECTION=	1
TARGET_DIR=	target
DEBUG_BUILD=	$(TARGET_DIR)/debug/$(BINARY)
DOC_BUILD=	$(TARGET_DIR)/doc/$(BINARY)
RELEASE_BUILD=	$(TARGET_DIR)/release/$(BINARY)

# Build debug binary
$(DEBUG_BUILD):
	$(CARGO) build

# Generate docs
$(DOC_BUILD):
	$(CARGO) doc --no-deps

# Build release binary
$(RELEASE_BUILD):
	$(CARGO) build --release

# Build debug binary
.PHONY: build
build: $(DEBUG_BUILD)

# Clean everything
.PHONY: clean
clean:
	$(CARGO) clean

# Clean only docs
.PHONY: cleandoc
cleandoc:
	$(CARGO) clean --doc

# Build debug binary
.PHONY: debug
debug: $(DEBUG_BUILD)

# Generate docs
.PHONY: doc
doc: $(DOC_BUILD)

# Lint the man page
.PHONY: manlint
manlint:
	$(MANDOC) \
		-T lint \
		$(MAN_DIR)/$(BINARY).$(MAN_SECTION)

# List outdated crates
.PHONY: outdated
outdated:
	$(CARGO) outdated

# Build release binary
.PHONY: release
release: $(RELEASE_BUILD)

# Run all tests
.PHONY: test
test:
	$(CARGO) test

# Run individual feature tests before main test.
.PHONY: test_all
test_all: test_cloudwatch test_s3 test

# Test CloudWatch feature alone
.PHONY: test_cloudwatch
test_cloudwatch:
	$(CARGO) test \
		--no-default-features \
		--features="cloudwatch"

# Test S3 feature alone
.PHONY: test_s3
test_s3:
	$(CARGO) test \
		--no-default-features \
		--features="s3"

# Update Cargo.lock
.PHONY: update
update:
	$(CARGO) update
