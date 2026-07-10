# LavaTimer（熔岩计时器）

LavaTimer 是一款常驻 macOS 菜单栏的多项目专注计时器。它使用 Tauri 2 构建，提供可拖动的透明悬浮窗口、紧凑胶囊视图、每日目标和本地专注记录。

## 功能

- 为多个项目分别计时并设置每日目标
- 在完整面板、紧凑胶囊和设置视图之间切换
- 展示今日进度、近 7 天专注时长和连续打卡天数
- 自动保存计时状态，关闭应用后重新打开仍可继续统计
- 菜单栏显示当前计时，并支持点击显示或隐藏窗口
- 窗口靠近屏幕边缘时自动吸附，并根据空间选择展开方向

## 技术栈

- [Tauri 2](https://v2.tauri.app/)
- Rust
- Vanilla HTML、CSS 和 JavaScript

当前版本主要面向 macOS；Tauri 配置中使用了菜单栏、透明窗口和 macOS 私有 API。

## 本地开发

请先安装：

- Node.js 18 或更高版本
- Rust stable 工具链
- macOS Xcode Command Line Tools

安装依赖并启动开发版：

```bash
npm install
npm run dev
```

## 检查与构建

运行 Rust 测试：

```bash
npm run check
```

构建 macOS 应用和 DMG：

```bash
npm run build
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 数据存储

项目设置、每日计时和历史记录保存在应用 WebView 的本地存储中。卸载应用或清除 WebView 数据前，请注意这些记录不会同步到云端。

## 许可证

本项目采用 [MIT License](LICENSE)。
