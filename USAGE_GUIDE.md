# EROFS 镜像模糊测试工具使用指南

> 本指南面向 EROFS/Linux 内核开发者，帮助您快速上手模糊测试工具。
>
> **前置知识**：熟悉 C 语言和 Linux 内核开发，但对 Rust 和模糊测试概念可能不太熟悉。

## 目录

1. [快速上手（5分钟）](#快速上手5分钟)
2. [工具概述](#工具概述)
3. [核心概念解释](#核心概念解释)
4. [环境准备](#环境准备)
5. [用户态测试](#用户态测试)
6. [内核态测试](#内核态测试)
7. [精准注入模式](#精准注入模式)
8. [输出结果解读](#输出结果解读)
9. [常见问题](#常见问题)
10. [Web UI 启动与访问](#web-ui-启动与访问)

---

## 快速上手（5分钟）

### 方式一：用户态测试（推荐新手）

测试用户态 `erofsfuse` 解析器，无需编译内核：

```bash
# 1. 编译项目
cargo build --release

# 2. 准备种子（有效的 EROFS 镜像）
mkdir -p seeds
echo "test" > /tmp/test.txt
mkfs.erofs -E noinline_data seeds/test.erofs /tmp/test.txt

# 3. 安装 erofsfuse（如果没有）
sudo apt install erofs-utils

# 4. 运行测试（100次迭代）
./target/release/erofs-fuzzer \
    --seeds ./seeds \
    --iterations 100 \
    --timeout 30
```

### 方式二：内核态测试（发现内核漏洞）

测试 Linux 内核 EROFS 驱动，需要编译测试内核：

```bash
# 1. 编译测试内核（约 15-30 分钟）
./scripts/build_kernel.sh

# 2. 运行内核态测试
./target/release/erofs-fuzzer \
    --seeds ./seeds \
    --executor qemu \
    --kernel ./kernel_build/bzImage \
    --initramfs ./kernel_build/rootfs.cpio.gz \
    --qemu-path /usr/bin/qemu-system-x86_64 \
    --iterations 10 \
    --timeout 120
```

> 说明：`cargo build --release` 现在会同时编译 Rust 后端和 `web-ui` 前端，
> `cargo build --features web --release` 已废弃，不再需要。

### 方式三：启动 Web UI（任务管理控制台）

```bash
# 1) 构建（会自动构建并内嵌前端）
cargo build --release

# 2) 启动 Web 控制台（默认 8080）
./target/release/erofs-fuzzer --web --web-port 8080

# 3) 浏览器访问
# http://127.0.0.1:8080
```

常用接口自检：

```bash
curl http://127.0.0.1:8080/api/health
```

---

## 工具概述

### 这个工具能做什么？

| 测试目标 | 测试内容 | 发现的漏洞类型 |
|---------|---------|---------------|
| **用户态** erofsfuse | 用户空间 EROFS 解析代码 | 内存越界、空指针、UAF |
| **内核态** EROFS 驱动 | Linux 内核 fs/erofs/ 代码 | 内核崩溃、内存损坏、KASAN 报告 |

### 工作原理（类比说明）

```
┌─────────────────────────────────────────────────────────────┐
│  如果您熟悉内核开发，可以这样理解：                            │
│                                                             │
│  传统测试：您写一个 test_case.c，调用函数，检查返回值         │
│                                                             │
│  模糊测试：工具自动生成成千上万个随机输入，尝试触发崩溃        │
│           类似 syzkaller，但专注于 EROFS 文件系统格式        │
│                                                             │
│  本工具特点：                                                │
│  - 结构感知：理解 EROFS 超级块、inode 等结构，不是随机字节   │
│  - 覆盖引导：优先测试未覆盖的代码路径                        │
│  - ASan 集成：自动检测内存错误                              │
└─────────────────────────────────────────────────────────────┘
```

### 与 syzkaller 的对比

| 特性 | 本工具 | syzkaller |
|------|--------|-----------|
| 测试方式 | 文件系统镜像注入 | 系统调用生成 |
| 测试目标 | EROFS 文件解析 | 任意内核子系统 |
| 结构感知 | EROFS 格式专用 | 通用系统调用描述 |
| 用户态测试 | ✅ 支持 | ❌ 仅内核 |
| 上手难度 | 低（无需编写描述文件） | 高（需要编写描述） |

---

## 核心概念解释

### 1. 种子（Seed）—— 测试起点

种子是一个**有效的** EROFS 镜像文件。模糊测试器从种子出发，通过变异生成新的测试用例。

**类比**：种子就像一个"正常"的测试用例，工具会对其"做手脚"来测试边界情况。

```bash
# 创建种子的最简单方法
mkdir -p seeds
mkdir -p /tmp/content
echo "hello" > /tmp/content/file.txt
mkfs.erofs seeds/basic.erofs /tmp/content/
```

**好种子的特点**：
- 覆盖不同文件类型（普通文件、目录、符号链接）
- 包含压缩数据（如果启用压缩）
- 包含扩展属性（xattr）
- 深层目录结构

### 2. 变异（Mutation）—— 自动化的"破坏"

变异器对种子进行智能修改，生成可能触发漏洞的输入。

| 变异器 | 目标结构 | 等效的 C 代码类比 |
|--------|---------|------------------|
| `ErofsSuperblockMutator` | 超级块 | 修改 `struct erofs_super_block` |
| `ErofsInodeMutator` | inode | 修改 `struct erofs_inode_compact` |
| `ErofsDirectoryMutator` | 目录项 | 修改 `struct erofs_dirent` |
| `ErofsXattrMutator` | 扩展属性 | 修改 xattr 头和条目 |

### 3. 执行器（Executor）—— 运行测试

| 执行器 | 测试目标 | 崩溃检测 |
|--------|---------|---------|
| `ErofsfuseExecutor` | 用户态 erofsfuse | 进程退出码、信号、ASan |
| `QemuKernelExecutor` | 内核 EROFS 驱动 | 内核 panic、Oops、KASAN |

---

## 环境准备

### 必需依赖

```bash
# Rust 工具链（编译本工具）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 基本编译工具
sudo apt install build-essential

# 前端构建工具（cargo build --release 会自动调用）
sudo apt install nodejs npm

# EROFS 工具（创建种子）
sudo apt install erofs-utils
```

### 用户态测试依赖

```bash
# erofsfuse（用户态解析器）
sudo apt install erofs-utils libfuse3-dev

# 可选：ASan 版本（检测更多内存问题）
git clone https://git.kernel.org/pub/scm/linux/kernel/git/xiang/erofs-utils.git
cd erofs-utils
CC=clang CFLAGS="-fsanitize=address -fno-omit-frame-pointer -g" \
  LDFLAGS="-fsanitize=address" ./configure --enable-fuse
make -j$(nproc)
# erofsfuse 位于 fuse/erofsfuse
```

### 内核态测试依赖

```bash
# QEMU
sudo apt install qemu-system-x86

# 内核编译依赖
sudo apt install flex bison bc libelf-dev libssl-dev

# 编译测试内核（约 15-30 分钟）
./scripts/build_kernel.sh
```

---

## 用户态测试

### 基本命令

```bash
# 编译
cargo build --release

# 运行（快速测试）
./target/release/erofs-fuzzer \
    --seeds ./seeds \
    --iterations 1000 \
    --timeout 30
```

`cargo build --release` 会自动完成前后端联合构建。

### 使用 ASan 版本检测更多问题

```bash
# 使用 ASan 编译的 erofsfuse
./target/release/erofs-fuzzer \
    --seeds ./seeds \
    --erofsfuse-path ./erofs-utils/fuse/erofsfuse \
    --asan \
    --iterations 10000
```

### 命令行参数速查

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--seeds` | 必需 | 种子目录 |
| `--output` | `./crashes` | 崩溃输出目录 |
| `--iterations` | 0（无限） | 最大迭代次数 |
| `--timeout` | 60 | 单次执行超时（秒） |
| `--workers` | 1 | 并行进程数 |
| `--erofsfuse-path` | `erofsfuse` | erofsfuse 路径 |
| `--asan` | false | 启用 ASan 检测 |
| `--log-level` | info | 日志级别 |

---

## 内核态测试

内核态测试通过 QEMU 运行编译的测试内核，将变异后的 EROFS 镜像作为块设备挂载，测试内核 EROFS 驱动。

### 测试内核特性

编译的测试内核包含以下调试选项：

| 选项 | 用途 |
|------|------|
| `CONFIG_KASAN` | 内核地址消毒器，检测内存错误 |
| `CONFIG_KCOV` | 代码覆盖率收集 |
| `CONFIG_EROFS_FS_DEBUG` | EROFS 调试输出 |
| `CONFIG_DEBUG_KMEMLEAK` | 内存泄漏检测 |
| `CONFIG_LOCKDEP` | 锁依赖检测 |

### 运行内核态测试

```bash
# 编译测试内核（首次需要）
./scripts/build_kernel.sh

# 运行 fuzzer
./target/release/erofs-fuzzer \
    --seeds ./seeds \
    --output ./crashes_kernel \
    --executor qemu \
    --kernel ./kernel_build/bzImage \
    --initramfs ./kernel_build/rootfs.cpio.gz \
    --qemu-path /usr/bin/qemu-system-x86_64 \
    --timeout 120 \
    --iterations 100
```

### 单个镜像测试

使用脚本快速测试单个镜像：

```bash
# 测试单个镜像
./scripts/run_qemu_test.sh ./seeds/test.erofs

# 指定内核和内存
KERNEL=./kernel_build/bzImage \
ROOTFS=./kernel_build/rootfs.cpio.gz \
MEMORY=1024 \
./scripts/run_qemu_test.sh ./crashes/crash-xxx.erofs

# 启用调试输出
./scripts/run_qemu_test.sh ./test.erofs --debug
```

### 检测结果

脚本会自动检测：
- **Kernel Panic** — 内核崩溃
- **Kernel Oops** — 内核错误
- **EROFS Error** — EROFS 相关错误
- **Call Trace** — 调用栈追踪
- **KASAN Report** — 内存错误报告

### 内核态测试参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--executor` | `erofsfuse` | 执行器类型：`erofsfuse` 或 `qemu` |
| `--kernel` | `./kernel_build/bzImage` | 内核镜像路径 |
| `--initramfs` | `./kernel_build/rootfs.cpio.gz` | initramfs 路径 |
| `--qemu-path` | `qemu-system-x86_64` | QEMU 可执行文件路径 |
| `--qemu-memory` | 512 | QEMU 内存大小（MB） |

### 性能优化建议

由于测试内核包含调试选项，启动较慢：

1. **增加超时时间**：`--timeout 120` 或更多
2. **编译快速内核**：修改 `scripts/build_kernel.sh`，禁用 KASAN
3. **减少种子大小**：使用小文件作为种子

---

## 精准注入模式

精准注入模式允许精确控制变异的位置，适用于针对性测试特定字段。

### 基本用法

```bash
# 精准变异 superblock.checksum 字段
./target/release/erofs-fuzzer \
    --seeds ./seeds \
    --target superblock.checksum \
    --strategy bitflip \
    --iterations 10000

# 精准变异 inode.i_size 字段
./target/release/erofs-fuzzer \
    --seeds ./seeds \
    --target inode.i_size \
    --strategy boundary

# 使用绝对偏移范围
./target/release/erofs-fuzzer \
    --seeds ./seeds \
    --range 1024:8 \
    --strategy zero
```

### 支持的目标字段

**超级块字段** (`struct erofs_super_block`)：
- `superblock.magic` — 魔数
- `superblock.checksum` — 校验和
- `superblock.blkszbits` — 块大小位数
- `superblock.rootnid` — 根目录 inode 号
- `superblock.meta_blkaddr` — 元数据块地址
- `superblock.feature_compat` / `feature_incompat` — 特性标志

**inode 字段** (`struct erofs_inode_compact`)：
- `inode.i_format` — 格式标志
- `inode.i_mode` — 文件模式
- `inode.i_size` — 文件大小
- `inode.i_uid` / `i_gid` — 用户/组 ID

### 变异策略

| 策略 | 说明 | 类比 |
|------|------|------|
| `bitflip` | 翻转随机位 | `data[offset] ^= (1 << bit)` |
| `arithmetic` | 算术变异 | `value += delta` |
| `interesting` | 边界值 | 使用 0, -1, INT_MAX 等 |
| `boundary` | 预定义边界值 | 0, 0xFF, 0xFFFF, 0xFFFFFFFF |
| `random` | 随机字节 | `rand_bytes()` |
| `zero` | 填充 0x00 | `memset(ptr, 0, size)` |
| `max` | 填充 0xFF | `memset(ptr, 0xFF, size)` |

---

## 输出结果解读

### 目录结构

```
crashes/
├── crash-000001a2b3c4d5e6-signal-11.erofs    # 触发崩溃的镜像
├── crash-000001a2b3c4d5e6-signal-11.log       # 崩溃详情
├── crash-000002b4c5d6e7f8-kernel-panic.erofs  # 内核崩溃
├── crash-000002b4c5d6e7f8-kernel-panic.log
└── ...

corpus/
├── erofs_input_abc123.erofs   # 发现新路径的输入
└── ...
```

### 崩溃类型

| 文件名后缀 | 含义 | 用户态 | 内核态 |
|-----------|------|--------|--------|
| `signal-11` | SIGSEGV（段错误） | ✅ | ❌ |
| `signal-6` | SIGABRT（assert/ASan） | ✅ | ❌ |
| `asan` | ASan 内存错误 | ✅ | ❌ |
| `kernel-panic` | 内核崩溃 | ❌ | ✅ |
| `kernel-oops` | 内核错误 | ❌ | ✅ |

### 分析崩溃

#### 用户态崩溃

```bash
# 直接重现
erofsfuse crashes/crash-xxx-signal-11.erofs /mnt/test

# 使用 GDB 调试
gdb --args erofsfuse crashes/crash-xxx-signal-11.erofs /mnt/test

# 使用 ASan 获取详细信息
ASAN_OPTIONS=symbolize=1 ./erofs-utils/fuse/erofsfuse \
    crashes/crash-xxx-signal-11.erofs /mnt/test
```

#### 内核态崩溃

```bash
# 在 QEMU 中重现
./scripts/run_qemu_test.sh crashes/crash-xxx-kernel-panic.erofs --debug

# 查看完整日志
cat crashes/crash-xxx-kernel-panic.log
```

---

## 常见问题

## Web UI 启动与访问

### 最简流程

```bash
cargo build --release
./target/release/erofs-fuzzer --web --web-port 8090
```

访问地址：

- `http://127.0.0.1:8080`
- `http://<你的机器IP>:8080`

### 常用参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--web` | false | 启动 Web 控制台模式 |
| `--web-port` | `8080` | Web 服务端口 |

### 启动后快速验证

```bash
curl http://127.0.0.1:8080/api/health
```

预期返回：

```json
{"status":"ok","version":"0.1.0"}
```

### Q1: erofsfuse 找不到

```bash
# 方法一：安装系统包
sudo apt install erofs-utils

# 方法二：指定完整路径
./target/release/erofs-fuzzer \
    --erofsfuse-path /usr/bin/erofsfuse \
    --seeds ./seeds
```

### Q2: 挂载失败

```bash
# 检查 FUSE 权限
ls -la /dev/fuse

# 添加用户到 fuse 组
sudo usermod -aG fuse $USER
# 注销后重新登录
```

### Q3: QEMU 测试超时

内核启动较慢，增加超时时间：

```bash
./target/release/erofs-fuzzer \
    --executor qemu \
    --timeout 180 \
    --seeds ./seeds
```

### Q4: 编译测试内核失败

```bash
# 检查磁盘空间（需要约 15GB）
df -h

# 检查依赖
sudo apt install flex bison bc libelf-dev libssl-dev

# 手动构建
cd kernel_build/linux-6.8
make -j$(nproc) bzImage
```

### Q5: 如何查看测试进度？

```bash
# 使用 debug 日志级别
./target/release/erofs-fuzzer \
    --log-level debug \
    --seeds ./seeds
```

### Q6: 测试速度太慢

```bash
# 减少变异次数
./target/release/erofs-fuzzer \
    --mutations-per-input 2 \
    --seeds ./seeds

# 增加并行度（用户态）
./target/release/erofs-fuzzer \
    --workers 4 \
    --seeds ./seeds
```

---

## 项目结构

```
erofs-image-injector-rs/
├── erofs-fuzzer/           # 主程序
│   ├── src/
│   │   ├── main.rs         # 入口点
│   │   ├── cli.rs          # 命令行参数
│   │   ├── fuzzer.rs       # 模糊测试主循环
│   │   ├── executor.rs     # 用户态执行器
│   │   └── qemu_executor.rs # 内核态执行器
│   └── Cargo.toml
│
├── erofs-format/           # EROFS 格式定义（类似内核头文件）
│   ├── src/
│   │   ├── superblock.rs   # struct erofs_super_block
│   │   ├── inode.rs        # struct erofs_inode_compact
│   │   ├── directory.rs    # struct erofs_dirent
│   │   └── xattr.rs        # 扩展属性
│   └── Cargo.toml
│
├── erofs-mutator/          # 变异器
│   ├── src/
│   │   ├── bitflip_mutator.rs
│   │   ├── superblock_mutator.rs
│   │   ├── inode_mutator.rs
│   │   ├── directory_mutator.rs
│   │   └── xattr_mutator.rs
│   └── Cargo.toml
│
├── scripts/
│   ├── build_kernel.sh     # 构建测试内核
│   └── run_qemu_test.sh    # 运行 QEMU 测试
│
├── kernel_build/           # 内核构建输出
│   ├── bzImage             # 测试内核
│   └── rootfs.cpio.gz      # initramfs
│
├── seeds/                  # 种子目录
├── crashes/                # 崩溃输出
└── corpus/                 # 语料库
```

---

## 对比：Rust vs C 语法速查

| Rust | C | 说明 |
|------|---|------|
| `let x: i32 = 0;` | `int x = 0;` | 变量声明 |
| `let mut x = 0;` | `int x = 0;` | 可变变量 |
| `struct Foo { ... }` | `struct foo { ... };` | 结构体 |
| `impl Foo { ... }` | `void foo_method() { }` | 方法实现 |
| `Option<T>` | `T*`（可空） | 可选值 |
| `Result<T, E>` | 返回值 + errno | 错误处理 |
| `match x { ... }` | `switch (x) { ... }` | 模式匹配 |
| `Vec<T>` | `T*` + size | 动态数组 |
| `cargo build` | `make` | 编译命令 |
| `cargo test` | `make test` | 测试命令 |

---

*文档版本：2.0*
*最后更新：2026-03-28*
