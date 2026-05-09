# docscan

基于终端的文档搜索工具，拥有丰富的 TUI 界面。
支持在 **Word**（`.docx` / `.doc` / `.wps`）、
**Excel**（`.xlsx` / `.xls` / `.et`）、**PDF** 和 **纯文本** 文件中
并行搜索关键词。

![主界面截图](https://pic1.imgdb.cn/item/69ff5ef78a9a8c274c5c255b.png)

## 功能特性

- **多格式支持** — Word、Excel、PDF、纯文本（`.txt`、`.md`、`.rs`、
  `.py`、`.js`、`.json`、`.csv`、`.log` 等）
- **多线程并行** — 基于 Rayon，可调并发数
- **丰富的 TUI** — 基于 Ratatui + Crossterm；键盘驱动，同时支持鼠标
- **目录浏览器** — 可视化目录选择器，用于添加扫描路径
- **实时过滤** — 按文件类型或路径文本过滤结果
- **命令模式** — `:type`、`:filter`、`:dir`、`:threads` 等
- **可点击 UI** — 按钮、可滚动列表、类型切换均支持鼠标交互
- **可扩展扫描器** — 实现 `Scanner` trait 即可添加新文件格式

## 安装

```bash
cargo install --git https://github.com/sprchu/docscan.git
```

## 使用

```bash
# 交互式 TUI（扫描当前目录）
docscan

# 带初始查询
docscan -q "关键词"

# 指定扫描目录
docscan /path/to/docs /another/path

# 启用 PDF 支持并设置线程数
docscan --pdf -j 8 -q "预算" ~/Documents
```

### 命令行选项

| 参数     | 说明                       |
| -------- | -------------------------- |
| `-q`     | 初始搜索关键词             |
| `-j`     | 线程数（默认：CPU 核心数） |
| `--pdf`  | 启用 PDF 扫描（默认关闭）  |
| `[dirs]` | 一个或多个扫描目录         |

## 快捷键

### 普通模式

| 按键      | 功能                                       |
| --------- | ------------------------------------------ |
| `Tab`     | 切换焦点（参数 → 路径 → 结果）             |
| `↑` / `↓` | 在当前面板内导航                           |
| `←` / `→` | 移动光标 / 调整数值                        |
| `Enter`   | 开始扫描（配置面板）、打开文件（结果面板） |
| `:`       | 进入命令模式                               |
| `Esc`     | 清除过滤文本                               |
| 字符键    | 输入关键词 / 过滤文本                      |

### 命令模式（`:`）

| 命令              | 功能                                              |
| ----------------- | ------------------------------------------------- |
| `:q`              | 退出                                              |
| `:k <关键词>`     | 设置搜索关键词                                    |
| `:j <n>`          | 设置线程数                                        |
| `:type <类型>`    | 切换文件类型（`word` / `excel` / `pdf` / `text`） |
| `:filter <类型>`  | 按类型过滤结果（或 `clear` 清除）                 |
| `:dir add <路径>` | 添加扫描目录                                      |
| `:dir rm [n]`     | 按序号删除目录                                    |
| `:dir browse`     | 打开目录浏览器                                    |
| `:dir clear`      | 清空所有目录                                      |

### 浏览模式

| 按键                  | 功能                 |
| --------------------- | -------------------- |
| `↑` / `↓` / `j` / `k` | 浏览目录列表         |
| `Enter`               | 进入目录 / 确认      |
| `Backspace` / `←`     | 返回上级目录         |
| `Space`               | 确认选择             |
| `~`                   | 跳转到主目录         |
| `/`                   | 跳转到文件系统根目录 |
| `Esc`                 | 取消                 |

## 截图

### 目录浏览弹窗

![目录浏览](https://pic1.imgdb.cn/item/69ff5ef68a9a8c274c5c255a.png)

### 带过滤的扫描结果

![扫描结果](https://pic1.imgdb.cn/item/69ff5ee68a9a8c274c5c2557.png)

## 项目结构

```
src/
├── main.rs          ─ 入口、CLI 参数、事件循环
├── utils.rs         ─ 杂项工具
├── command.rs       ─ 命令执行
├── ui.rs            ─ 终端渲染（Ratatui）
│
├── app/             ─ 应用逻辑，按职责拆分
│   ├── types.rs     ─ 数据类型（FileType、Focus、Mode 等）
│   ├── state.rs     ─ App 结构体 + 构造函数
│   ├── config.rs    ─ 查询、线程数、文件类型切换
│   ├── results.rs   ─ 结果选择、过滤
│   ├── browse.rs    ─ 目录浏览器
│   ├── cmd.rs       ─ 命令模式文本编辑
│   ├── input.rs     ─ 键盘事件分发
│   └── mouse.rs     ─ 鼠标事件分发
│
└── scan/            ─ 可插拔扫描器架构
    ├── mod.rs       ─ Scanner trait + 调度器
    ├── docx.rs      ─ .docx 扫描器
    ├── doc.rs       ─ .doc / .wps 二进制扫描器
    ├── excel.rs     ─ .xlsx / .xls / .et 扫描器
    ├── pdf.rs       ─ PDF 扫描器
    └── text.rs      ─ 纯文本扫描器
```

## 依赖

| 库                                                     | 用途                   |
| ------------------------------------------------------ | ---------------------- |
| [Ratatui](https://ratatui.rs)                          | 终端 UI 框架           |
| [Crossterm](https://github.com/crossterm-rs/crossterm) | 终端控制 + 输入        |
| [Rayon](https://github.com/rayon-rs/rayon)             | 并行迭代               |
| [WalkDir](https://github.com/BurntSushi/walkdir)       | 递归文件系统遍历       |
| [Calamine](https://github.com/tafia/calamine)          | Excel 读取             |
| [lopdf](https://github.com/J-F-Liu/lopdf)              | PDF 读取               |
| [zip](https://github.com/zip-rs/zip2)                  | ZIP/DOCX 读取          |
| [cfb](https://github.com/mdsteele/rust-cfb)            | 复合文档二进制格式读取 |
| [memchr](https://github.com/BurntSushi/memchr)         | 快速字节模式搜索       |

## 许可证

MIT — 详见 [LICENSE](LICENSE)。
