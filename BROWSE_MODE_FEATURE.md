# TUI Autocomplete 浏览模式功能说明

## 新增功能

当使用 `@` 文件自动补全功能时，现在支持**浏览模式**：

- 输入 `@` 后，使用**上下箭头键**导航时，输入框会实时显示当前选中文件的完整路径
- 这样你可以清楚地看到将要选择哪个文件
- 继续输入字符会自动退出浏览模式，恢复编辑过滤文本

## 使用场景

### 场景 1：浏览并选择文件
1. 输入 `@exa`
2. 按 **Down** 箭头多次
3. 输入框会显示每个选中文件的完整路径（如 `@examples/`、`@src/main.rs` 等）
4. **重要：** 路径前会保留 `@` 符号
5. 找到想要的文件后按 **Enter** 确认

### 场景 2：浏览后继续编辑
1. 输入 `@exa`
2. 按 **Down** 箭头浏览（进入浏览模式）
3. 决定继续输入，按 `m` 键
4. 自动退出浏览模式，恢复到 `@exam` 并更新过滤列表

### 场景 3：浏览后删除
1. 输入 `@examples`
2. 按 **Up** 箭头浏览（进入浏览模式）
3. 按 **Backspace**
4. 自动退出浏览模式，恢复到 `@examples` 然后删除一个字符

## 技术实现

### 修改的文件
- `coding-agent/src/ui/tui/input.rs`

### 新增字段
```rust
browse_mode_user_input: Option<String>  // 存储浏览模式时用户的输入文本
```

### 新增方法
```rust
update_textarea_with_selection()  // 用选中文件路径更新输入框
```

### 修改的行为
1. **上下箭头**：进入浏览模式，保存用户输入，显示选中文件路径
2. **输入字符**：退出浏览模式，恢复用户输入，继续编辑
3. **Backspace**：退出浏览模式，恢复用户输入，执行删除
4. **Enter**：使用当前选中的文件路径

## 测试建议

### 编译和运行
```bash
cd /mnt/workspace/coding-agent
cargo build --release
./target/release/coding-agent
```

### 测试步骤

1. **基本浏览测试**
   - 输入 `@exa`
   - 按 Down 箭头多次
   - 观察输入框是否显示不同的文件路径
   - 按 Enter 选择文件

2. **浏览后继续输入**
   - 输入 `@exa`
   - 按 Down 箭头
   - 输入 `m`
   - 确认显示 `@exam` 且过滤列表更新

3. **浏览后删除**
   - 输入 `@examples`
   - 按 Up 箭头
   - 按 Backspace
   - 确认恢复到 `@examples` 然后删除字符

4. **边界情况**
   - 输入 `@` 立即按箭头（无过滤）
   - 浏览到列表末尾并循环
   - 浏览后按 Enter 选择目录

## 预期结果

✅ 箭头导航时输入框显示选中文件路径
✅ 继续输入时平滑退出浏览模式
✅ Backspace 后平滑退出浏览模式
✅ Enter 在浏览和编辑模式下都正常工作
✅ 无视觉闪烁或文本损坏

## Git 提交建议

```bash
git add coding-agent/src/ui/tui/input.rs
git commit -m "Add browse mode to TUI autocomplete for live preview

- Add browse_mode_user_input field to track user's filter text
- Add update_textarea_with_selection() to show selected file in textarea
- Up/Down arrows now display selected file path while navigating
- Typing or Backspace exits browse mode and restores user input
- Improves UX by showing what will be selected before pressing Enter"
```
