# Coding Agent 项目说明

## 项目信息
- **仓库名**: coding-agent
- **远程地址**: git@github.com:JoshuaXuDa/coding-agent.git
- **主分支**: main
- **工作目录**: /mnt/workspace/coding-agent

## 项目结构
- `mvp.md` - MVP 文档
- `PROJECT_NOTES.md` - 本项目说明文件
- `samples/` - 示例文件（不提交到 git）

## Git 工作流
**重要规则**: 每次修改代码后必须提交到 git

提交命令：
```bash
git add -A
git commit -m "描述性提交信息

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>"
git push
```

## .gitignore 配置
```
samples/
```

## 注意事项
1. ✅ 所有代码修改必须提交到 git
2. ❌ samples 目录的内容不会被提交
3. 🔑 使用 GitHub SSH 密钥进行推送
