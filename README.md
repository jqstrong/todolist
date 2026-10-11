# 稿纸 · 桌面待办

一个键盘唤起的桌面待办清单。按一次全局快捷键，面板出现在屏幕最前面；写完按下回车，
再按一次快捷键，它就收起来。Windows / Tauri 2 + Vue 3。

界面是黑白的，只有一种颜色——靛蓝，而且只用在「完成」上：方框被填满、勾被描出来、
一道笔迹划过那行字。设计说明与取舍记录见 [`docs/design-notes.md`](docs/design-notes.md)。

## 用法

面板出现时光标已经在召唤行里，全流程不用碰鼠标：

| 按键 | 作用 |
| --- | --- |
| `回车` | 记下正在写的那件事（新事项排在最上面） |
| `↑` `↓` | 在列表里上下选择；从第一项再往上就回到召唤行 |
| `空格` | 完成 / 取消完成选中的事 |
| `E` 或 `回车` | 编辑选中的事（`回车` 保存，`ESC` 放弃，清空内容即删除） |
| `DEL` | 删除选中的事 |
| `ALT` + `↑` `↓` | 调整顺序 |
| `CTRL` + `Z` | 撤销删除或清除 |
| `ESC` | 收起面板（若召唤行里有没写完的字，先擦掉它） |
| 直接打字 | 选中某一项时直接敲字，会回到召唤行接着写 |

鼠标也能用：点方框完成，点文字选中，双击编辑，右侧 `×` 删除，拖动顶部一行移动窗口。

## 召唤与收起

- **默认全局快捷键 `Ctrl + Alt + Space`**：在任何一个界面按下，面板出现在最前面并拿到焦点；
  再按一次收起。若这个组合被别的程序占用，应用会在设置里说明，换一个即可。
- 失去焦点自动收起（可在设置里关掉，这样它能一直留在屏幕上）。
- `ESC` 收起。
- 关闭窗口不等于退出：应用常驻托盘，托盘菜单是「显示 / 隐藏」和「退出」，左键单击托盘图标也能切换面板。
- 重复启动只会唤出已经开着的那个窗口。

## 设置

面板右上角的调节图标打开设置：召唤快捷键（点一下直接按新的组合键录入）、失焦收起、
始终置顶、开机自启、已完成事项是否留在列表里、外观（跟随系统 / 浅色 / 深色）、打开数据文件夹。

## 数据

待办存在一个 JSON 文件里，随手就能备份：

```
%APPDATA%\com.jqstrong.gaozhi\data.json     任务与设置
%APPDATA%\com.jqstrong.gaozhi\window.json   窗口位置
```

写入是原子的（先写临时文件再替换），所以断电或强杀不会留下半个文件。数据损坏时按默认值启动，
不会崩。

## 开发

```bash
pnpm install
pnpm tauri dev                 # 开发（会自己起 Vite）
pnpm build                     # 只做类型检查和前端构建
pnpm tauri build --no-bundle   # 编译 release 可执行文件，不打安装包
```

> **打包一律用 `pnpm tauri build`，不要直接 `cargo build`。** cargo 盯的是 Rust 源码的
> mtime，前端产物（`dist/`）和图标（`src-tauri/icons/`）变了它未必认为要重新编译，
> 结果就是 exe 里还是上一版界面。换图标之后若发现 exe 里仍是旧图标，
> 先 `cargo clean -p todolist --release` 再打包。

纯前端预览（不需要 Rust）：`pnpm dev`，然后打开 http://localhost:1420 。
浏览器里会自动降级成 localStorage 存数据，并塞几条样例；
`?demo=empty`、`?demo=long`、`?theme=night` 可以看空列表、长列表和夜间样式。

## 目录

```
src/
  App.vue                   面板本体：状态、键盘模型、把各部件接起来
  bridge.ts                 与 Tauri 的唯一通道（浏览器里自动降级）
  types.ts                  与 Rust 一一对应的数据契约
  composables/useTodos.ts   任务状态、顺序、撤销、落盘
  components/
    AppHead.vue             顶部一行（日期 / 计数 / 设置），也是拖动窗口的把手
    CaptureRow.vue          召唤行——界面的主角
    TaskRow.vue             一条待办，以及“划掉”那道笔迹
    HintBar.vue             底部图例 / 状态行
    SettingsSheet.vue       设置
    ToggleSwitch.vue        设置里的开关
  styles/theme.css          设计令牌与基础层
src-tauri/src/
  lib.rs                    入口：插件、托盘、状态
  commands.rs               暴露给前端的命令
  hotkey.rs                 加速键解析与注册
  window.rs                 显示/隐藏/置顶/位置记忆
  store.rs                  原子读写磁盘
```
