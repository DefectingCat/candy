PACKAGE_MANAGER = pnpm

all: pack

nvm:
	fnm use && $(CMD)

deps:
	make nvm CMD="$(PACKAGE_MANAGER) i"

fix:
	make nvm CMD="$(PACKAGE_MANAGER) i && $(PACKAGE_MANAGER) pretty"

pack:
	cp -r docs/* i18n/zh-Hans/docusaurus-plugin-content-docs/current \
		&& make nvm CMD="$(PACKAGE_MANAGER) i && $(PACKAGE_MANAGER) build"

# build:

.PHONY: all
