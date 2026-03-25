# TUI Autocomplete 修复说明

## 修复内容
已修复 TUI 文件自动补全功能中的多个索引越界错误。

## 修改的文件
- `coding-agent/src/ui/tui/autocomplete.rs`

## 具体修复

### 1. 第 398 行：安全截断长文件名
**问题：** 直接切片 `chars[..37]` 在字符数少于 37 时会 panic
**修复：** 添加边界检查 `37.min(chars.len())`

### 2. 第 432-465 行：安全的匹配高亮切片
**问题：** 高亮匹配文本时，`char_pos`、`match_end` 可能超出数组边界
**修复：** 在每次切片前添加边界检查：
- `char_pos > 0 && char_pos <= total_chars`
- `char_pos < match_end && char_pos <= total_chars && match_end <= total_chars`
- `match_end < total_chars && match_end <= total_chars`

### 3. 第 310 行：parent_directory 的安全切片
**问题：** `rfind('/')` 返回的位置在切片时可能越界
**修复：** 添加显式边界检查 `if pos + 1 <= self.input_prefix.len()`

### 4. input.rs 第 264 行：已有保护
**验证：** Backspace 处理器已有正确的边界检查 `if new_text.len() > trigger_pos + 1`

## 如何测试

### 1. 编译项目
```bash
cd /mnt/workspace/coding-agent
cargo build --release
```

### 2. 运行 TUI 并测试以下场景

#### 测试场景 1：快速输入
- 输入 `@exa` 快速输入，观察是否 panic
- 输入 `@examples` 快速输入

#### 测试场景 2：慢速输入
- 输入 `@` 然后逐个字符输入：`e` `x` `a` `m` `p` `l` `e` `s`

#### 测试场景 3：删除操作
- 输入 `@examples/` 然后按多次 Backspace
- 输入 `@exa` 然后按 Backspace 删除

#### 测试场景 4：目录导航
- 输入 `@examples` 然后按 Enter 进入目录
- 进入目录后按 Backspace 返回上级目录

#### 测试场景 5：上下导航
- 输入 `@` 然后按 Up/Down 箭头导航

#### 测试场景 6：极端情况
- 输入 `@` 匹配非常短的文件名（1-2 个字符）
- 输入 `@` 匹配非常长的文件名（>40 个字符）

### 3. 预期结果
- ✅ 所有场景都不应该出现 panic
- ✅ 自动补全弹窗正常显示
- ✅ 文件导航流畅
- ✅ 匹配文本高亮正常显示

### 4. 如果遇到问题
如果仍然出现 panic，请记录：
1. 具体的输入步骤
2. 错误信息的完整内容
3. 截图

## Git 提交建议
```bash
git add coding-agent/src/ui/tui/autocomplete.rs
git commit -m "Fix multiple index out of bounds panics in TUI autocomplete

- Add bounds checking to display name truncation (line 398)
- Add bounds checking to highlight slicing (lines 432-465)
- Add bounds checking to parent_directory string slicing (line 310)
- Verify input.rs Backspace handler has proper protection

Fixes panics when typing after @ trigger, navigating, or deleting
characters in the autocomplete popup."
```
