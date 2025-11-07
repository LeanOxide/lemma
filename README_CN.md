# Lemma - 现代化的 Lean4 工具链管理器

[English](README.md) | [简体中文](README_CN.md)

**Lemma** 是 [elan](https://github.com/leanprover/elan) 的重写版本，解决了关键的可用性问题，特别是代理支持和自定义工具链源的问题。

## 核心特性

### 完整的代理支持
- **支持 HTTP、HTTPS 和 SOCKS5 代理**

### 自定义源和镜像
- 配置自定义注册表 URL

```toml
release_url = "https://release.custom.org"
```

## 安装

### 快速安装（推荐）

**Linux / macOS：**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://lemma.puqing.work/install.sh | sh
```

或者先下载并检查脚本：

```bash
curl --proto '=https' --tlsv1.2 -sSfL https://lemma.puqing.work/install.sh -o install.sh
chmod +x install.sh
./install.sh
```

**Windows (PowerShell)：**

```powershell
irm https://lemma.puqing.work/install.ps1 | iex
```

或者先下载并检查脚本：

```powershell
Invoke-WebRequest -Uri https://lemma.puqing.work/install.ps1 -OutFile install.ps1
.\install.ps1
```

### 从源码构建

```bash
# 构建发布版本
cargo build --release

# 安装
cargo install --path .
```

### 更新

安装完成后，你可以使用下面脚本更新 lemma

```bash
lemma self update
```

该命令会检查最新版本，如果有更新版本则自动下载。

## 使用方法

### 基本命令

```bash
# 安装工具链
lemma toolchain install stable
lemma toolchain install nightly
lemma toolchain install v4.0.0

# 列出已安装的工具链
lemma toolchain list

# 设置默认工具链
lemma default stable

# 更新工具链
lemma update

# 显示信息
lemma info

# 更新/卸载
lemma self update              # 更新 lemma 本身
lemma self uninstall           # 卸载 lemma 及所有工具链
```

## 配置文件

Lemma 将配置存储在 `~/.lemma/config.toml`（或 `$LEMMA_HOME/config.toml`）中。

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
- `LEMMA_HOME` - Lemma 安装目录（默认：`~/.lemma`）
- `LEMMA_RELEASE_URL` - 覆盖默认的发布服务器

## 换源

[上交大镜像源](https://s3.jcloud.sjtu.edu.cn/899a892efef34b1b944a19981040f55b-oss01/elan/leanprover/mirror_clone_list.html) 虽然提供了 Lean 工具链的镜像，但并未提供 release 索引，所以本项目提供了一个自定义的 release 索引，供国内用户使用。（如果落后于镜像可以发送issue或者邮箱告知）。

编辑 `~/.lemma/config.toml`，将 `release_url` 修改为：

```toml
release_url = "https://lemma.puqing.work"
```

## 贡献

欢迎贡献！需要改进的关键领域：

1. **测试** - 添加全面的测试覆盖
2. **文档** - 扩展用户和开发者文档
3. **平台支持** - 在 Windows、macOS、Linux 上测试

## 许可证

[MIT]

## 致谢

- **Elan** - Lean 工具链管理器

---

**注意：** Lemma 正处于早期开发阶段。虽然核心基础设施已经就绪，但工具链安装尚未完全实现。使用需自担风险。
