# 🍺 酒要点点 (Beer Mouse Clicker)

<p align="center">
  <img src="icon.ico" width="128" alt="icon" />
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" /></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/rust-1.70%2B-orange.svg" /></a>
  <a href="https://github.com/cjxpj/beer_mouse_clicker/releases"><img src="https://img.shields.io/badge/platform-Windows%2010%2F11-brightgreen.svg" /></a>
  <a href="https://github.com/cjxpj/beer_mouse_clicker/releases"><img src="https://img.shields.io/github/v/release/cjxpj/beer_mouse_clicker" /></a>
</p>

<p align="center">
  <a href="README.md">🇺🇸 English</a>
</p>

**酒要点点** 是一款 Windows 平台的键鼠宏自动化工具，基于 Rust + egui 构建。支持可视化编排鼠标点击、键盘输入、延迟、识图、滚轮等 **22 种任务动作**，可录制、循环、后台运行，是替代重复性桌面操作的效率利器。

---

## ✨ 特性

- **可视化任务编排** — 添加、编辑、删除、拖拽排序任务步骤，流畅动画交互
- **22 种任务动作** — 覆盖鼠标、键盘、延迟、识图、滚轮、通知、程序启动等全场景操作
- **录制回放** — 通过低级鼠标钩子录制点击、移动、滚轮和键盘操作，自动检测长按与组合键
- **热键控制** — 自定义热键（默认 `F6`），在任意窗口触发任务执行/停止
- **安全锁** — 执行时可选锁定键盘和鼠标，防止误触干扰
- **系统托盘** — 关闭窗口缩至托盘，右键菜单切换窗口/退出，热键后台响应
- **开机自启** — 支持注册表自启，可分别配置"启动执行"与"开机执行"
- **图色识别** — 基于 NCC 归一化互相关的屏幕模板匹配，支持限定窗口搜索与可信度阈值
- **中英双语** — 自动检测系统语言，运行时可切换
- **Catppuccin Mocha 暗色主题** — 优雅暗色界面，自动加载中文字体

---

## 📋 任务动作类型（22 种）

| 动作 | 说明 |
|------|------|
| 鼠标点击 | 在指定坐标执行点击 |
| 鼠标按下 | 在指定坐标按下鼠标按键 |
| 鼠标松开 | 在指定坐标松开鼠标按键 |
| 鼠标移动（绝对） | 将鼠标移动到绝对坐标 |
| 鼠标移动（相对） | 将鼠标移动相对偏移量 |
| 鼠标移动（缓动） | 缓动动画移动鼠标到目标位置 |
| 鼠标移动（窗口居中） | 将鼠标移动到目标窗口中心 |
| 按键 | 按下并松开单个按键 |
| 组合键 | 按下组合键（如 `Ctrl+C`） |
| 按键按下 | 按下按键不松开 |
| 按键松开 | 松开指定按键 |
| 滚轮 | 在指定位置滚动滚轮 |
| 延迟 | 等待固定时间（毫秒） |
| 随机延迟 | 在范围内随机等待 |
| 整点等待 | 等待到指定时间点 |
| 识图 | 屏幕模板匹配，找到后执行后续操作 |
| 等待按键 | 等待用户按下指定按键后继续 |
| 等待输入 | 等待用户输入（显示输入框） |
| 通知 | 弹出通知消息 |
| 剪贴板文本 | 将文本写入剪贴板并粘贴 |
| 打开程序 | 启动外部程序 |
| 显示/隐藏窗口 | 控制主窗口的显示状态 |

---

## 🎬 截图

<!-- 截图占位：将截图放在仓库根目录或 docs/ 下，然后引用 -->
<!-- ![主界面](screenshots/main.png) -->
<!-- ![任务编辑](screenshots/edit.png) -->

---

## 📦 安装

从 [Releases](https://github.com/cjxpj/beer_mouse_clicker/releases) 页面下载最新 `beer_mouse_clicker.exe`，双击运行即可（绿色免安装）。

> 💡 首次运行后会自动在 exe 同目录生成 `beer_clicker.bmc`（SQLite 数据库），用于保存任务、配置和图片。

### 开机自启

```bash
beer_mouse_clicker.exe --autostart
```

该命令将程序注册到注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`，开机时自动以托盘模式启动。

---

## 🛠️ 构建

### 前置要求

- [Rust 工具链](https://rustup.rs)（MSVC 目标，1.70+）
- Windows 10 / 11

### 编译

```bash
# 克隆仓库
git clone https://github.com/cjxpj/beer_mouse_clicker.git
cd beer_mouse_clicker

# 确保项目根目录存在 icon.ico

# 编译 Release
cargo build --release
```

生成的 exe 位于 `target/release/beer_mouse_clicker.exe`。

---

## 🕹️ 使用说明

### 基本操作

| 操作 | 方式 |
|------|------|
| 添加任务 | 点击下方 **＋** 按钮，选择任务类型并配置参数 |
| 编辑任务 | 点击任务项右侧 **编辑** 按钮 |
| 删除任务 | 点击任务项右侧 **删除** 按钮 |
| 排序任务 | 拖拽任务项左侧手柄 |
| 开始执行 | 按下热键（默认 `F6`） |
| 停止执行 | 再次按下热键 |
| 录制 | 勾选 **录制**，然后操作鼠标键盘自动生成任务 |

### 配置项

| 配置 | 默认值 | 说明 |
|------|--------|------|
| 任务间隔 | `1000ms` | 每轮任务间的等待时间 |
| 热键 | `F6` | 开始/停止任务的热键 |
| 锁键盘 | 关闭 | 执行时屏蔽所有键盘输入 |
| 锁鼠标 | 关闭 | 执行时将鼠标限制在 1×1 像素范围 |
| 任务循环 | 开启 | 是否循环执行任务列表 |
| 后台模式 | 关闭 | 始终在后台运行（关闭窗口缩至托盘） |
| 录制压缩 | 开启 | 合并连续鼠标移动事件 |
| 开机自启 | 关闭 | 开机时自动启动 |
| 启动执行 | 关闭 | 程序启动时自动开始执行任务 |
| 开机执行 | 关闭 | 开机自启时自动开始执行任务 |

---

## 🏗️ 技术栈

| 技术 | 用途 |
|------|------|
| [Rust](https://www.rust-lang.org) | 编程语言 |
| [egui / eframe](https://github.com/emilk/egui) | GUI 框架 |
| [rusqlite](https://github.com/rusqlite/rusqlite) | SQLite 数据持久化 |
| [winapi](https://github.com/retep998/winapi-rs) | Windows API 调用（输入注入、钩子、截图、注册表） |
| [image](https://github.com/image-rs/image) | 图片加载与缩略图 |
| [regex](https://github.com/rust-lang/regex) | 正则匹配 |
| [rand](https://github.com/rust-random/rand) | 随机数生成 |
| [rfd](https://github.com/PolyMeilex/rfd) | 原生文件对话框 |
| [winreg](https://github.com/gentoo90/winreg-rs) | Windows 注册表操作 |

---

## 🤝 贡献

欢迎提交 Issue 和 Pull Request。

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 打开 Pull Request

---

## 👤 作者

- QQ: 2960965389
- GitHub: [@cjxpj](https://github.com/cjxpj)
- QQ群: 310345976

---

## 📄 许可

MIT License — 详见 [LICENSE](LICENSE)
