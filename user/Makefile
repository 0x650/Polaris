CFILES := $(wildcard *.c)
ASMFILES := $(wildcard *.asm)
OBJFILES := $(CFILES:.c=.o) $(ASMFILES:.asm=.o)

override DEFAULT_CC := gcc
$(eval $(call DEFAULT_VAR,CC,$(DEFAULT_CC)))

override DEFAULT_LD := ld
$(eval $(call DEFAULT_VAR,LD,$(DEFAULT_LD)))

override DEFAULT_CFLAGS := -g -O2 -pipe
$(eval $(call DEFAULT_VAR,CFLAGS,$(DEFAULT_CFLAGS)))

override DEFAULT_NASMFLAGS := -F dwarf -g
$(eval $(call DEFAULT_VAR,NASMFLAGS,$(DEFAULT_NASMFLAGS)))

override DEFAULT_LDFLAGS :=
$(eval $(call DEFAULT_VAR,LDFLAGS,$(DEFAULT_LDFLAGS)))

LDFLAGS := -Tlinker.ld
NASMFLAGS := -felf64
CFLAGS := -ffreestanding -mno-red-zone -Os -pipe -Wall -Wextra -std=gnu11 -mno-sse -mno-sse2

.PHONY: all
all: user

user: $(OBJFILES)
	$(LD) $(LDFLAGS) $^ -o user.elf

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.asm
	nasm $(NASMFLAGS) $< -o $@

.PHONY: clean
clean:
	rm -rf *.elf $(OBJFILES)
