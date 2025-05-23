default: check
alias a := all
alias b := build
alias c := check
alias f := fmt
alias fc := fmt-check
alias i := install
alias l := lint
alias lf := lint-fix
alias r := run
alias t := test

aud:
  cargo audit --all-targets

build:
  cargo build

check:
  cargo check --all-targets

fmt:
  cargo fmt --all

fmt-check:
  cargo fmt --all -- --check

install:
  cargo install --path .

lint:
  cargo clippy --all-targets

lint-fix:
  cargo clippy --fix  --allow-dirty --allow-staged

publish-dry:
  cargo publish --dry-run --allow-dirty

run:
  cargo run

test:
  cargo nextest run

all:
  cargo fmt --all
  cargo clippy --all-targets
  cargo nextest run
