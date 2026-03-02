# Candy Documentation

基于 [Docusaurus 3](https://docusaurus.io/) 构建的 Candy 文档站点。

## 开发

```bash
# 安装依赖
pnpm install

# 启动开发服务器
pnpm start
```

开发服务器启动后自动打开浏览器窗口，支持热重载。

## 构建

```bash
pnpm build
```

生成静态文件至 `build` 目录，可部署至任意静态文件服务器。

## 部署

### GitHub Pages

```bash
# 使用 SSH
USE_SSH=true pnpm deploy

# 使用 HTTPS
GIT_USER=<username> pnpm deploy
```

### 其他平台

构建后将 `build` 目录部署至任意静态托管服务（Vercel、Netlify、Cloudflare Pages 等）。

## 目录结构

```
docs/
├── docs/          # 文档源文件
│   ├── intro.md
│   ├── quick-start.md
│   ├── faq.md
│   └── config/    # 配置相关文档
├── blog/          # 博客文章
├── src/           # 自定义组件与样式
├── static/        # 静态资源
└── docusaurus.config.ts
```

## 添加文档

1. 在 `docs/` 目录创建 Markdown 文件
2. 更新 `sidebars.ts` 添加侧边栏入口
3. 使用相对路径引用图片等资源

## 配置

主配置文件 `docusaurus.config.ts` 包含：

- 站点元数据（标题、URL、描述）
- 导航栏与页脚
- 主题与样式
- 插件配置

详细配置参考 [Docusaurus 官方文档](https://docusaurus.io/docs/configuration)。