# ==========================================
# 变量定义区 (把又长又臭的路径封装起来，方便以后修改)
# ==========================================
TARGET      := riscv64gc-unknown-none-elf
MODE        := release
KERNEL_ELF  := target/$(TARGET)/$(MODE)/os
KERNEL_BIN  := $(KERNEL_ELF).bin
BOOTLOADER  := ../bootloader/rustsbi-qemu.bin

# 告诉 Make，如果在终端只敲 `make`，默认执行 `run` 目标
.DEFAULT_GOAL := run

# ==========================================
# 指令目标区
# ==========================================

# 1. 编译源码生成 ELF 文件
build:
	cargo build --release

# 2. 剥离元数据生成 .bin 二进制文件
# 注意：冒号后面的 build 表示“依赖关系”。即执行 bin 之前，必须先保证 build 成功。
bin: build
	rust-objcopy --strip-all $(KERNEL_ELF) -O binary $(KERNEL_BIN)

# 3. 启动 QEMU 运行内核
# 同样，依赖于 bin，保证每次运行的都是最新脱壳的内核代码
run: bin
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=0x80200000

# 4. 一键清理编译缓存（送你的附加实用功能）
clean:
	cargo clean