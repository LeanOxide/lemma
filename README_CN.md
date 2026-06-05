# Lemma - 现代化的 Lean4 工具链管理器

[English](README.md) | [简体中文](README_CN.md)

**Lemma** 是 [elan](https://github.com/leanprover/elan) 的重写版本，重点改善代理支持、自定义工具链源和受限网络环境下的可用性。

## 核心特性

### 完整的代理支持

- **支持 HTTP、HTTPS 和 SOCKS5 代理**
- 遵循标准环境变量：`HTTP_PROXY`、`HTTPS_PROXY`、`NO_PROXY`

### 自定义源和镜像

可以配置自定义 Lean release index：

```toml
release_url = "https://release.custom.org"
```

## 安装

### 快速安装（推荐）

Lemma 现在通过 Python 包发布，包名和命令名都是 `lemma`。

```bash
pipx install lemma
```

如果不使用 `pipx`，可以使用 Python user site 安装：

```bash
python -m pip install --user lemma
```

Windows 可以使用 Python launcher：

```powershell
py -m pip install --user lemma
```

安装后，运行 `lemma toolchain install stable` 等初始化命令。Lemma 会在 `~/.lemma/bin` 下创建 `lean`、`lake`、`leanc` 等代理命令；如果希望直接运行这些代理命令，请把该目录加入 `PATH`。

### 从源码构建

```bash
# 构建发布版本
cargo build --release -p lemma

# 从当前源码安装 CLI
cargo install --path crates/lemma-rs
```

### 更新

使用安装 Lemma 时的同一个包管理器更新：

```bash
pipx upgrade lemma
# 或
python -m pip install --user --upgrade lemma
```

`lemma self update` 会显示这些安全的包管理器更新命令，不会直接覆盖当前正在运行的二进制文件。

## 使用方法

### 基本命令

```bash
# 安装 Lean 工具链
lemma toolchain install stable
lemma toolchain install nightly
lemma toolchain install v4.0.0

# 列出工具链
lemma toolchain list

# 设置默认工具链
lemma default stable

# 更新已安装的 channel 工具链
lemma toolchain upgrade

# 显示当前工具链信息
lemma show

# 更新/卸载
lemma self update              # 显示包管理器更新命令
lemma self uninstall           # 删除 Lemma 管理的工具链和 ~/.lemma 数据
```

所有工具链管理操作统一使用 `lemma toolchain ...`。

## 配置文件

Lemma 将配置存储在 `~/.lemma/lemma.toml`（或 `$LEMMA_HOME/lemma.toml`）中。

配置示例：

```toml
version = "1"
default_toolchain = "leanprover/lean4:stable"
path_setup_shown = true
release_url = "https://release.lean-lang.org"

[overrides]
```

## 环境变量

Lemma 遵循标准的代理环境变量：

- `HTTP_PROXY` / `http_proxy` - HTTP 代理 URL
- `HTTPS_PROXY` / `https_proxy` - HTTPS 代理 URL
- `ALL_PROXY` / `all_proxy` - 所有协议的代理
- `NO_PROXY` / `no_proxy` - 绕过代理的域名列表（逗号分隔）
- `LEMMA_HOME` - Lemma 管理目录（默认：`~/.lemma`）
- `LEMMA_RELEASE_URL` - 覆盖 Lean release index URL
- `LEMMA_TOOLCHAIN` - 为当前会话覆盖活动工具链

## 换源

如果需要使用自定义 Lean release index，可以编辑 `~/.lemma/lemma.toml`：

```toml
release_url = "https://mirror.example.com/lean-releases"
```

或者使用环境变量：

```bash
export LEMMA_RELEASE_URL=https://mirror.example.com/lean-releases
```

## 常见问题

### 命令找不到

如果找不到 `lemma` 命令，请确认 Python 包管理器的 scripts 目录在 `PATH` 中。使用 pipx 时可以运行：

```bash
pipx ensurepath
```

如果找不到 `lean`、`lake`、`leanc` 等代理命令，请确认 Lemma 的代理目录在 `PATH` 中：

```bash
export PATH="$HOME/.lemma/bin:$PATH"
```

### 工具链未安装

```bash
lemma toolchain list
lemma toolchain install stable
```

## 贡献

欢迎贡献！需要改进的关键领域：

1. **测试** - 添加全面的测试覆盖
2. **文档** - 扩展用户和开发者文档
3. **平台支持** - 在 Windows、macOS、Linux 上测试

## 许可证

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)

## 致谢

- **Elan** - Lean 工具链管理器

---

**注意：** Lemma 正处于早期开发阶段。使用前建议先在非关键环境中测试。
