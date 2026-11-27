# cool_fox

[English](README.md) | **简体中文**

[![CI](https://github.com/hmaoraze/cool_fox/actions/workflows/ci.yml/badge.svg)](https://github.com/hmaoraze/cool_fox/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/hmaoraze/cool_fox?label=release)](https://github.com/hmaoraze/cool_fox/releases/latest)

面向 Xbox Series（Scarlett / GDK）x64 可执行文件的实验性兼容层与着色器翻译运行时，在 Windows D3D12（AMD Radeon 与 NVIDIA GeForce）上原生运行。

> **法律声明：** 本仓库不包含任何受版权保护的游戏代码、密钥、素材或可执行文件，不提供也不链接任何第三方软件，不指导、不鼓励绕过主机保护。请只用于你有权运行的软件。此处所有原创代码以 **GPL-3.0-or-later** 授权（见 [LICENSE](LICENSE) 与 [docs/asset-policy.md](docs/asset-policy.md)）。

---

## 这个分支是什么

运行时是上游的工作：[DoubleGhosty/cool_fox](https://github.com/DoubleGhosty/cool_fox) 完成了移植、翻译与测量。本分支是叠在它之上的**发行与可用性层**，改动集中在五处，每一处都对应"某次运行起不来"或"某次运行无法解释"的具体问题：

- **六个具名预设，只写一遍**（`apps/victory/src/profile.rs`）。三个 `.cmd` 过去各自携带一份开关表，已经彼此不一致、也和二进制不一致。现在启动器只给名字，`victory profiles` 打印名字的含义。
- **Windows 不会替你选的那张卡。** `D3D12CreateDevice(None)` 拿到的是"默认适配器"，双显卡笔记本上那是核显——游戏要么跑在错卡上，要么永远过不了 Shader Model 6.6 检查。运行时改为按 DXGI 性能顺序选择，`victory gpu-info` 打印选中的是哪张。
- **一个窗口**（`apps/victory-gui`）：把文件夹拖进去、选预设、点 Start。每个按钮背后都是同一条命令行。
- **Release 是构建出来的，不是手攒的。** 打 `v*` 标签即以 `--features agility-sdk` 构建两个可执行文件，从 NuGet 取微软的 `D3D12Core.dll`、从 runner 取 CRT，打包并作为附件挂上——一个包同时服务 Windows 10 与 Windows 11。
- **CI 在你下载任何东西之前给出结论**：两套特性下的 `clippy -D warnings` 加全量测试，跑在仓库 pin 的 toolchain 上。

这个分支**刻意不做**的事：再加一套关于开关的主张，或者承诺提交线程给不出的帧率。帧时间由那条线程决定，不由显卡决定——详见下文。

---

## 主要特性与近期改进

- **Pareto 内存驻留与 32 GB 内存支持：** 自适应的显存驻留上限，避免 16–32 GB 机器上的 WDDM 分页耗尽（`0x8007000E`）。
- **管线与描述符稳定性：**
  - 启动器中设 `VICTORY_PIPELINE_WAIT_MS=0` 与 `VICTORY_LIBRARY_STORE_LATER=1`，消除 500 ms 卡顿与 200 ms 以上的管线库保存停顿。
  - 抗 panic 的管线工作队列（`Pending::make_with`），单次翻译失败不再挂住等待线程。
  - `submit_split` 动态释放描述符槽位（`list::tables_done`）与 `alias_of` 引用钉住（`Held` 守卫），避免异步纹理别名期间的槽位耗尽与释放后使用。
  - DirectStorage 暂存缓冲清理，以及空闲列表/上传裁剪（`VICTORY_LIST_IDLE_TRIM_S`）。
  - 无绑定堆根签名标志位（`CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED`、`SAMPLER_HEAP_DIRECTLY_INDEXED`）改为启动时探测，在拒绝它们的运行时上省略，修掉旧 D3D12 运行时上的 "Unsupported bit-flag set"。
  - 离线批量着色器翻译器（`victory shaders`）按单个着色器捕获 panic，一个空值断言不再毁掉整轮翻译。
- **着色器与光追修复：**
  - 着色器 LOD 槽位修复（数组纹理坐标查询里保留未使用的 LOD 槽位为未定义，以通过 DXIL 校验）。
  - Wave32 级联下沉、作用域化的水下着色器下沉，以及光追 BVH 描述符的动态视图地址缓冲索引（`SLOT_VIEW_ADDRESSES`）。
  - 体素可见性 Wave64 掩码来源恢复，以及混形预派发初始化。
- **NVIDIA 与视频同步：** 解码视频帧拷贝的同步刷新（为 NVIDIA 兼容，启动器默认 `VICTORY_THIN=0`、`VICTORY_JOIN_PARTS=0`），以及常量环境光（`VICTORY_AMBIENT=constant:0.8`）。
- **Wine / Proton 与 DXIL 健壮性：** 为 `vkd3d-shader` 代码生成根签名、单锁存 DXIL 循环 CFG 归一化、256 MiB 管线工作线程栈。
- **自定义分辨率、缩放与帧率：**
  - `VICTORY_OUTPUT=1920x1080`（或 `2560x1440`）与 `VICTORY_RENDER_SCALE=0.667..1.0`。
  - `VICTORY_FRAME_INTERVAL_MS=16.667`（60 FPS）或 `8.333`（120 FPS），同时模拟时序保持 1.00x。
- **渲染端口（性能）：**
  - 启动器默认开启 `VICTORY_PORT_GUARD=1` 与 `VICTORY_PORT_JOURNAL=1`：写守卫锁共享与日志拆分，在所有测过的显卡上每帧省 2–5 ms。
  - `VICTORY_PORT_CONSTANTS=1` 与 `VICTORY_PORT_CBV=1` 仍为可选，保证 8 GB NVIDIA 的稳定；A 卡用户应使用 `Play-VIctory-AMD.cmd`，它开启全部端口。
  - 默认开启 `VICTORY_LEVELS_ON_DEMAND=1`：只为游戏实际持有的 mip 层分配显存，最多省 3 GB。

---

## 怎么跑

### 环境要求
- Windows 10/11 x64
- 支持 AVX2 的 CPU
- 支持 Shader Model 6.6 与 Mesh Shader 的 Direct3D 12 显卡
- Rust 工具链（见 `rust-toolchain.toml`）与 MSVC C++ 生成工具（含 Windows SDK）

### 从源码构建

```powershell
git clone https://github.com/DoubleGhosty/cool_fox.git
cd cool_fox
cargo build --release --locked -p victory
```

在 **Windows 10** 上请改用 D3D12 Agility SDK 构建。Windows 10 自带的 D3D12 没有 shader model 6.6 的无绑定堆，根签名会以 `0x80070057` 失败：

```powershell
cargo build --release --locked -p victory --features agility-sdk
powershell -ExecutionPolicy Bypass -File scripts\get-agility-sdk.ps1
```

该特性让 `victory.exe` 请求放在它旁边 `D3D12\` 里的微软 `D3D12Core.dll` 1.619.6，脚本会从 NuGet 取来放好（用 `--features agility-sdk` 构建测试程序时也会放到 `target\debug\deps` 旁边）。这种构建必须有那个 DLL：缺了 D3D12 根本起不来（`0x887E0003`）。默认构建不受影响、也不需要它。Wine / Proton 不适用。

### 启动

```powershell
$env:VICTORY_GAME_DIR = 'D:\path\to\your\title'
.\Play-VIctory.cmd
```

AMD Radeon（RX 7000 以上、16 GB 以上显存）并开启全部渲染端口：

```powershell
.\Play-VIctory-AMD.cmd
```

4 GB 显存或小内存机器：

```powershell
.\Play-VIctory-4GB.cmd
```

| 启动器 | 目标 | 渲染端口 | 说明 |
|---|---|---|---|
| `Play-VIctory.cmd` | NVIDIA + AMD，8 GB 以上 | 6、7 开；1、5 需手动 | 所有显卡的安全默认 |
| `Play-VIctory-AMD.cmd` | AMD 16 GB 以上 | 1、5、6、7 全开 | 最激进；端口取自社区头部分支 |
| `Play-VIctory-4GB.cmd` | 4 GB 显存 | 6、7 开；1、5 关 | Series S 档，缩减卡端缓冲 |
| `scripts\play.ps1` | 诊断 | 6、7 开；`-Safe` 关闭 | 完整日志；`-RayTracing` 开光追 |

### 预设

开关表不再每个启动器写一遍。它住在 `apps/victory/src/profile.rs`，启动器只给名字：

```powershell
victory profiles                 # 全部表，按 --vendor other
victory profiles --vendor amd    # 同一批表，按 A 卡的口味
victory gpu-info                 # 本次运行会用哪张卡，以及为什么
```

| 预设 | 主机档 | 设了什么 |
|---|---|---|
| `pareto-residency` | Series X | `Play-VIctory.cmd` 一直以来那一套 |
| `pareto-residency-amd` | Series X | 同上，外加录制列表与端口 1、5 |
| `balanced` | Series X | 1080p / 0.667 渲染缩放 / 60 Hz 节拍，端口按厂商 |
| `speed` | Series X | 936p，不限帧，能开的端口全开 |
| `quality` | Series X | 1440p / 缩放 1.0 + `VICTORY_BVH_BUILDER`（慢约三分之一） |
| `lowvram` | Series S | 720p，更粗的卡端缓冲，关闭驱动后台工作 |

`balanced` 与 `speed` 会读厂商：`--vendor amd` 打开录制列表（`VICTORY_THIN`）与全部四个端口；`nvidia` 与 `other`（默认）把它们关掉，因为开了录制列表后，画面中人物的头部和皮肤不会被绘制，而 12 GB 以下开 `VICTORY_PORT_CBV` 可能掉路面。环境里已有的开关永远不会被覆盖，所以 `set VICTORY_AMBIENT=0.4` 仍然只改一个值。

启动器的第一个参数就是预设名：

```powershell
.\Play-VIctory.cmd balanced
.\Play-VIctory-AMD.cmd speed
```

`victory run` 也改为按 DXGI 性能顺序选择适配器，而不是 Windows 的默认适配器——后者在双显卡笔记本上是核显。若某台机器上这样选反而错，用 `VICTORY_D3D12_ADAPTER=0` 退回。

### 同一个命令行的窗口版

```powershell
cargo run --release -p victory-gui
```

把 dump 文件夹拖到窗口上（或粘贴路径）、选预设、点 Start。按钮就是 `victory run --profile …`、`victory shaders …`、`victory gpu-info` 与 `victory config`，下方输出窗格逐行就是它们的输出。它先在自身旁边找 `victory.exe`，然后找 `target\release`；`VICTORY_BIN` 可覆盖两者。



### 诊断与预热

```powershell
copy victory.example.toml victory.toml      # 本地路径放在仓库之外
cargo build --locked --profile diag -p victory --features diagnostics
scripts\play.ps1 -Exe <path_to_executable>
```

`scripts\play.ps1 -Safe` 关闭较新的实验路径；`-RayTracing` 开启光追间接光照。

要提前翻译着色器或预热已缓存的管线：

```powershell
target\diag\victory.exe shaders <archive_paths>
target\diag\victory.exe warm --used
```

`warm` 在有一次运行把管线记下来之前无事可做（`scripts\play.ps1 -Exe <path> -Record`）。有些开关是着色器缓存键的一部分，所以这两条命令必须看到实际运行时的那些开关：用 `scripts\play.ps1 -Warm` 与 `scripts\play.ps1 -Shaders <archive_paths>` 来跑，并带上与 play 相同的 `-Safe`、`-RayTracing`、`-More`。

命令、调试开关与技术笔记见 [docs/README-fxceless.md](docs/README-fxceless.md) 与 [docs/findings.md](docs/findings.md)。每个 `VICTORY_...` 环境变量也都在源码读取它的地方有说明。

---

## 性能调优（渲染端口）

每个渲染端口消掉一类每帧的 CPU 工作。它们先在 AMD 上测试，在显存紧张的 NVIDIA 上可能有边角情况。

| 端口 | 开关 | 作用 | 收益 | 默认 |
|---|---|---|---|---|
| 1 | `VICTORY_PORT_CONSTANTS=1` | 游戏那份 174 MB 常量堆由显卡就地读取，不再上传或填充 | 约 6 ms，每帧少拷约 17 MB | 关（8 GB NVIDIA：C++ 异常 #15） |
| 5 | `VICTORY_PORT_CBV=1` | 常量缓冲按原始字节读取，每帧消掉约 22,000 次填充 | 约 4.7 ms | 关（8 GB NVIDIA：丢面） |
| 6 | `VICTORY_PORT_GUARD=1` | 写守卫锁由缺页与刷新共享，每 64 KiB 块只占用一次 | 约 2 ms | **开** |
| 7 | `VICTORY_PORT_JOURNAL=1` | 日志备注拆成 64 份、各持锁；堆在提交时同步 | 约 1–3 ms | **开** |
| RT | `VICTORY_PORT_RT=1` | 优化光追提交顺序，填充由拷贝转发 | 开光追时消掉 15–61 ms 停顿 | 关 |

其它性能开关：
- `VICTORY_LEVELS_ON_DEMAND=1` —— 只为游戏持有的纹理层分配显存（最多省 3 GB）
- `VICTORY_THIN=1` —— 记录列表调用而非在游戏线程上执行（A 卡稳定；N 卡：头部皮肤可能不画）
- `VICTORY_JOIN_PARTS=8` —— 跨 8 条线程合并提交分片
- `VICTORY_UPLOAD_AHEAD=1` —— 在游戏请求之前投机上传纹理
- `VICTORY_CARD_BUFFERS_MIN_KB=64` —— 卡端缓冲拷贝的最小尺寸（越小，卡上缓冲越多）
- `VICTORY_CARD_STEPS=2` —— 缓冲填充放到 GPU，而不是卡住 CPU

---

## 架构要点

- **录制式命令列表（`VICTORY_THIN`）：** 列表调用在游戏线程上录制，再由工作线程翻译。
- **并行提交准备（`VICTORY_ASYNC_SUBMIT`、`VICTORY_JOIN_PARTS`）：** 帧准备、AVX2 比较、描述符记录与落地并行化，按提交顺序汇合。
- **写守卫中继（`VICTORY_RELAY`、`VICTORY_LAND`）：** GPU 写过的缓冲由一条专职中继线程按变更的双字回读。
- **卡端缓冲填充（`VICTORY_CARD_STEPS`）：** 依赖卡上字节的缓冲填充在 GPU 上执行，不再卡住 CPU。
- **专用纹理拷贝队列与回读：** 纹理在专用拷贝队列上传；GPU 写过的纹理由 CPU 回读时干净拷回。
- **帧记录与回放：** 逐 pass 计时与着色器变体比对，经 `VICTORY_RECORD`（[docs/replay.md](docs/replay.md)）。

---

## 社区与 Portwatch

[portwatch-vi.xyz](https://portwatch-vi.xyz/#top) 按进度给兼容层排名。截至 2026 年 11 月，主要开源分支及其关键技术：

| 项目 | 显卡目标 | 此处采纳的技术 |
|---|---|---|
| [Belobug](https://github.com/Belobug/VIctory-Belobug-AMD) | AMD | PORT_CBV、PORT_CONSTANTS、PORT_GUARD、PORT_JOURNAL、环境光、管线工作线程 |
| [Kelvaroq](https://github.com/stellarr-studio/kelvaroq) | NVIDIA | 统一内存层、异步 present、通过放大阶段实现 mesh、变更页查询 |
| [DexNom](https://github.com/DexNom/VIctory) | AMD | 光追、稀疏纹理、并行前奏、FSR 4.1.2 |
| [fxceless/victory-fork](https://github.com/fxceless/victory-fork) | NVIDIA | 着色器清理 pass、按需分配层、查找缓存、异步提交 |
| [avatarmanz](https://github.com/avatarmanz/VIctory-7900XTX) | AMD | 多线程录制、渲染端口、帧记录与回放 |

这些项目里 `cool_fox` 尚未引入、正在研究的技术：
- **统一内存层**（Kelvaroq）：GPU 查找直接在游戏内存上完成、不做拷贝，把每帧拷贝从 >100 MB 降到 <8 MB。
- **异步 present**（Kelvaroq）：最多提前两帧流水化，把 CPU 提交与 GPU 完成解耦。
- **描述符表打补丁**（Belobug）：存储自己报告写过的槽位，而不是每帧扫约 60 万个槽位。
- **通过放大阶段实现 mesh 绘制**（Kelvaroq）：消掉间接绘制的钳制 pass。

---

## 致谢与上游

本仓库建立在 VIctory 生态众多贡献者的工作之上，并保留提交历史与署名：

- **[VIctory](https://github.com/Marvisak/VIctory)**（Marvisak 与贡献者）：加载器、HLE 服务、D3D12 后端与着色器重编译器。
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**：基础分支与上游 PR。
- **codec-d / CGBJ**：`cool_fox` 运行时与 pareto 驻留架构。
- **[avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)**：多线程命令列表录制、帧率解锁与渲染端口。
- **[DexNom/VIctory](https://github.com/DexNom/VIctory)**：光追修复、立方体渐变、稀疏纹理与 Wave64/DXIL pass。
- **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**：环境光、管线工作线程与整数目标混合保护。
- **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**、**[myokex/victory-amd](https://github.com/myokex/victory-amd)**、**Geckomacabre**、**cxy0072019-sketch**。
- **[XCL](https://github.com/ivall/xcl)**（GPL-2.0-only）作为参考研究；未包含任何 XCL 代码。

---

## 许可证

GPL-3.0-or-later。见 [LICENSE](LICENSE) 与 [docs/asset-policy.md](docs/asset-policy.md)。
