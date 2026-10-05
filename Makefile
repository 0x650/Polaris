BUILDTYPE ?= debug
BUILDDIR ?= build/$(ARCH)-$(BUILDTYPE)
CARGO_FLAGS ?=

HOST_CC := cc
HOST_CFLAGS := -g -O2 -pipe -fno-stack-protector
HOST_CPPFLAGS :=
HOST_LDFLAGS :=
HOST_LIBS :=

CFLAGS := -fno-stack-protector

ifeq '$(BUILDTYPE)' 'release'
CARGO_BUILD_FLAGS ?= --release
else
CARGO_BUILD_FLAGS ?=
endif

ifeq '$(ARCH)' 'x86_64'
TARGET_JSON = toolchain/x86_64-kernel.json
TARGET_NAME = x86_64-kernel
else
$(error Unknown architecture)
endif

limine-binary/limine:
	rm -rf limine-binary
	curl -L https://github.com/Limine-Bootloader/Limine/releases/latest/download/limine-binary.tar.gz | gunzip | tar -xf -
	$(MAKE) -C limine-binary \
		CC="$(HOST_CC)" \
		CFLAGS="$(HOST_CFLAGS)" \
		CPPFLAGS="$(HOST_CPPFLAGS)" \
		LDFLAGS="$(HOST_LDFLAGS)" \
		LIBS="$(HOST_LIBS)"

polaris.iso: limine-binary/limine kernel user
	rm -rf iso_root
	mkdir -p iso_root/boot
	cp -v $(BUILDDIR)/polaris user/user.elf iso_root/boot/
	mkdir -p iso_root/boot/limine
	cp -v limine.conf limine-binary/limine-bios.sys limine-binary/limine-bios-cd.bin limine-binary/limine-uefi-cd.bin iso_root/boot/limine/
	mkdir -p iso_root/EFI/BOOT
	cp -v limine-binary/BOOTX64.EFI iso_root/EFI/BOOT/
	cp -v limine-binary/BOOTIA32.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs -R -r -J -b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table -hfsplus \
		-apm-block-size 2048 --efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o polaris.iso
	./limine-binary/limine bios-install polaris.iso
	rm -rf iso_root

.PHONY: kernel
kernel:
	mkdir -p $(BUILDDIR)
	cargo +nightly $(CARGO_FLAGS) build $(CARGO_BUILD_FLAGS) \
		-Zjson-target-spec \
		-Zbuild-std=core,alloc \
		-Zbuild-std-features=compiler-builtins-mem \
		--target=$(TARGET_JSON)
	cp target/$(TARGET_NAME)/$(BUILDTYPE)/polaris $(BUILDDIR)/

.PHONY: user
user:
	make -C user
