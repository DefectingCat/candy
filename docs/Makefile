PACKAGE_MANAGER = pnpm

all: pack

nvm:
	fnm use && $(CMD)

deps:
	make nvm CMD="$(PACKAGE_MANAGER) i"

fix:
	make nvm CMD="$(PACKAGE_MANAGER) i && $(PACKAGE_MANAGER) pretty"

pack:
	make nvm CMD="$(PACKAGE_MANAGER) i && $(PACKAGE_MANAGER) build"

.PHONY: all
